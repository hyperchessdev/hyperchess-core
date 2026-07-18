import { GameStore } from '../types/game-store';
import { GameRecord, GameQueryOptions, GameNotFoundError, Unsubscribe } from '../types/game-record';

/**
 * SQLite adapter for mobile and offline-first apps
 * Requires: npm install better-sqlite3
 *
 * Features:
 * - WAL mode for concurrent access
 * - Sync queue for offline-first architecture
 * - Automatic timestamp management
 * - Efficient local queries
 */
export class SqliteGameStore implements GameStore {
  private db: any; // better-sqlite3.Database
  private watchers: Map<string, Set<(game: GameRecord) => void>> = new Map();
  private pollIntervals: Map<string, any> = new Map();

  constructor(dbPath: string) {
    try {
      const Database = require('better-sqlite3');
      this.db = new Database(dbPath);

      // Enable WAL mode for better concurrent access
      this.db.pragma('journal_mode = WAL');
      this.db.pragma('synchronous = NORMAL');

      this.initSchema();
    } catch (error) {
      throw new Error('SQLite adapter requires "better-sqlite3" package: npm install better-sqlite3');
    }
  }

  private initSchema(): void {
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS games (
        id TEXT PRIMARY KEY,
        hpgn TEXT NOT NULL,
        hfen TEXT NOT NULL,
        players TEXT,
        metadata TEXT,
        result TEXT,
        user_id TEXT,
        synced BOOLEAN DEFAULT 0,
        created_at TEXT,
        updated_at TEXT
      );

      CREATE INDEX IF NOT EXISTS idx_user_id ON games(user_id);
      CREATE INDEX IF NOT EXISTS idx_created_at ON games(created_at DESC);
      CREATE INDEX IF NOT EXISTS idx_synced ON games(synced);
    `);
  }

  async saveGame(game: GameRecord): Promise<string> {
    const id = game.id || this.generateId();
    const now = new Date().toISOString();

    const stmt = this.db.prepare(`
      INSERT OR REPLACE INTO games (
        id, hpgn, hfen, players, metadata, result, user_id, synced, created_at, updated_at
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `);

    stmt.run(
      id,
      game.hpgn,
      game.hfen,
      game.players ? JSON.stringify(game.players) : null,
      game.metadata ? JSON.stringify(game.metadata) : null,
      game.result || null,
      game.userId || null,
      game.synced ?? 0, // Mark as unsynced by default
      game.createdAt || now,
      now
    );

    // Notify watchers
    const updated = await this.loadGame(id);
    this.watchers.get(id)?.forEach((cb) => cb(updated));

    return id;
  }

  async loadGame(id: string): Promise<GameRecord> {
    const stmt = this.db.prepare('SELECT * FROM games WHERE id = ?');
    const row = stmt.get(id);

    if (!row) {
      throw new GameNotFoundError(id);
    }

    return this.rowToRecord(row);
  }

  async deleteGame(id: string): Promise<void> {
    const stmt = this.db.prepare('DELETE FROM games WHERE id = ?');
    const result = stmt.run(id);

    if (result.changes === 0) {
      throw new GameNotFoundError(id);
    }

    this.stopWatching(id);
  }

  async listGames(options?: GameQueryOptions): Promise<GameRecord[]> {
    let query = 'SELECT * FROM games WHERE 1=1';
    const params: any[] = [];

    if (options?.userId) {
      query += ' AND user_id = ?';
      params.push(options.userId);
    }

    if (options?.result) {
      query += ' AND result = ?';
      params.push(options.result);
    }

    query += ` ORDER BY created_at ${options?.sort === 'asc' ? 'ASC' : 'DESC'}`;

    if (options?.limit) {
      query += ' LIMIT ?';
      params.push(options.limit);
    }

    if (options?.offset) {
      query += ' OFFSET ?';
      params.push(options.offset);
    }

    const stmt = this.db.prepare(query);
    const rows = stmt.all(...params);

    return rows.map((row: any) => this.rowToRecord(row));
  }

  async countGames(options?: GameQueryOptions): Promise<number> {
    let query = 'SELECT COUNT(*) as count FROM games WHERE 1=1';
    const params: any[] = [];

    if (options?.userId) {
      query += ' AND user_id = ?';
      params.push(options.userId);
    }

    if (options?.result) {
      query += ' AND result = ?';
      params.push(options.result);
    }

    const stmt = this.db.prepare(query);
    const result = stmt.get(...params) as any;

    return result.count;
  }

  watch(id: string, callback: (game: GameRecord) => void): Unsubscribe {
    if (!this.watchers.has(id)) {
      this.watchers.set(id, new Set());

      // Start polling
      const interval = setInterval(async () => {
        try {
          const game = await this.loadGame(id);
          const callbacks = this.watchers.get(id);
          callbacks?.forEach((cb) => cb(game));
        } catch {
          // Game deleted or doesn't exist
          this.stopWatching(id);
        }
      }, 1000); // Poll every 1 second

      this.pollIntervals.set(id, interval);
    }

    this.watchers.get(id)!.add(callback);

    return () => {
      const callbacks = this.watchers.get(id);
      if (callbacks) {
        callbacks.delete(callback);
        if (callbacks.size === 0) {
          this.stopWatching(id);
        }
      }
    };
  }

  private stopWatching(id: string): void {
    const interval = this.pollIntervals.get(id);
    if (interval) {
      clearInterval(interval);
      this.pollIntervals.delete(id);
    }
    this.watchers.delete(id);
  }

  async isHealthy(): Promise<boolean> {
    try {
      this.db.prepare('SELECT 1').get();
      return true;
    } catch {
      return false;
    }
  }

  // Offline-first sync management
  async getSyncQueue(): Promise<GameRecord[]> {
    const stmt = this.db.prepare('SELECT * FROM games WHERE synced = 0');
    return stmt.all().map((row: any) => this.rowToRecord(row));
  }

  async markSynced(id: string): Promise<void> {
    const stmt = this.db.prepare('UPDATE games SET synced = 1, updated_at = ? WHERE id = ?');
    stmt.run(new Date().toISOString(), id);
  }

  async clearSyncQueue(): Promise<void> {
    const stmt = this.db.prepare('UPDATE games SET synced = 1 WHERE synced = 0');
    stmt.run();
  }

  async exportAll(): Promise<GameRecord[]> {
    const stmt = this.db.prepare('SELECT * FROM games ORDER BY created_at DESC');
    return stmt.all().map((row: any) => this.rowToRecord(row));
  }

  async importGames(games: GameRecord[]): Promise<number> {
    let count = 0;

    for (const game of games) {
      try {
        await this.saveGame(game);
        count++;
      } catch (error) {
        console.warn(`Failed to import game ${game.id}:`, error);
      }
    }

    return count;
  }

  async clear(): Promise<number> {
    const stmt = this.db.prepare('DELETE FROM games');
    const result = stmt.run();
    this.watchers.clear();
    this.pollIntervals.forEach((interval) => clearInterval(interval));
    this.pollIntervals.clear();
    return result.changes;
  }

  private rowToRecord(row: any): GameRecord {
    return {
      id: row.id,
      hpgn: row.hpgn,
      hfen: row.hfen,
      players: row.players ? JSON.parse(row.players) : undefined,
      metadata: row.metadata ? JSON.parse(row.metadata) : undefined,
      result: row.result,
      userId: row.user_id,
      synced: !!row.synced,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
    };
  }

  private generateId(): string {
    return `game_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  close(): void {
    this.pollIntervals.forEach((interval) => clearInterval(interval));
    this.pollIntervals.clear();
    this.watchers.clear();
    this.db.close();
  }
}
