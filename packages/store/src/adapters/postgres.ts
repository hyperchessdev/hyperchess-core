import { GameStore } from '../types/game-store';
import { GameRecord, GameQueryOptions, GameNotFoundError, Unsubscribe } from '../types/game-record';

/**
 * PostgreSQL adapter for production game storage
 * Requires: npm install pg
 *
 * Features:
 * - Connection pooling
 * - LISTEN/NOTIFY for real-time updates
 * - Full-text search ready
 * - Automatic timestamp management
 */
export class PostgresGameStore implements GameStore {
  private pool: any; // pg.Pool
  private listeners: Map<string, Set<(game: GameRecord) => void>> = new Map();
  private client: any; // For LISTEN/NOTIFY

  constructor(connectionString: string) {
    // Dynamic import to make pg optional
    try {
      const { Pool } = require('pg');
      this.pool = new Pool({
        connectionString,
        max: 20,
        idleTimeoutMillis: 30000,
        connectionTimeoutMillis: 2000,
      });

      // Setup LISTEN for notifications
      this.setupNotifications();
    } catch (error) {
      throw new Error('PostgreSQL adapter requires "pg" package: npm install pg');
    }
  }

  private async setupNotifications(): Promise<void> {
    try {
      this.client = await this.pool.connect();
      await this.client.query('LISTEN game_updates');

      this.client.on('notification', (msg: any) => {
        try {
          const payload = JSON.parse(msg.payload);
          const callbacks = this.listeners.get(payload.id);
          if (callbacks) {
            callbacks.forEach((cb) => cb(payload.game));
          }
        } catch (error) {
          console.warn('Failed to parse notification:', error);
        }
      });
    } catch (error) {
      console.warn('Failed to setup notifications:', error);
    }
  }

  async saveGame(game: GameRecord): Promise<string> {
    const id = game.id || this.generateId();
    const now = new Date().toISOString();

    const query = `
      INSERT INTO games (
        id, hpgn, hfen, players, metadata, result, user_id, synced, created_at, updated_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
      ON CONFLICT (id) DO UPDATE SET
        hpgn = $2, hfen = $3, players = $4, metadata = $5,
        result = $6, synced = $8, updated_at = $10
      RETURNING id
    `;

    const result = await this.pool.query(query, [
      id,
      game.hpgn,
      game.hfen,
      game.players ? JSON.stringify(game.players) : null,
      game.metadata ? JSON.stringify(game.metadata) : null,
      game.result || null,
      game.userId || null,
      game.synced ?? true,
      game.createdAt || now,
      now,
    ]);

    // Notify listeners
    if (this.listeners.has(id)) {
      const updated = await this.loadGame(id);
      this.listeners.get(id)?.forEach((cb) => cb(updated));
    }

    return result.rows[0].id;
  }

  async loadGame(id: string): Promise<GameRecord> {
    const result = await this.pool.query('SELECT * FROM games WHERE id = $1', [id]);

    if (result.rows.length === 0) {
      throw new GameNotFoundError(id);
    }

    return this.rowToRecord(result.rows[0]);
  }

  async deleteGame(id: string): Promise<void> {
    const result = await this.pool.query('DELETE FROM games WHERE id = $1', [id]);

    if (result.rowCount === 0) {
      throw new GameNotFoundError(id);
    }

    this.listeners.delete(id);
  }

  async listGames(options?: GameQueryOptions): Promise<GameRecord[]> {
    let query = 'SELECT * FROM games WHERE 1=1';
    const params: any[] = [];
    let paramCount = 1;

    if (options?.userId) {
      query += ` AND user_id = $${paramCount}`;
      params.push(options.userId);
      paramCount++;
    }

    if (options?.result) {
      query += ` AND result = $${paramCount}`;
      params.push(options.result);
      paramCount++;
    }

    query += ` ORDER BY created_at ${options?.sort === 'asc' ? 'ASC' : 'DESC'}`;

    if (options?.limit) {
      query += ` LIMIT $${paramCount}`;
      params.push(options.limit);
      paramCount++;
    }

    if (options?.offset) {
      query += ` OFFSET $${paramCount}`;
      params.push(options.offset);
    }

    const result = await this.pool.query(query, params);
    return result.rows.map((row: any) => this.rowToRecord(row));
  }

  async countGames(options?: GameQueryOptions): Promise<number> {
    let query = 'SELECT COUNT(*) as count FROM games WHERE 1=1';
    const params: any[] = [];

    if (options?.userId) {
      query += ` AND user_id = $1`;
      params.push(options.userId);
    }

    if (options?.result) {
      query += ` AND result = $${params.length + 1}`;
      params.push(options.result);
    }

    const result = await this.pool.query(query, params);
    return parseInt(result.rows[0].count, 10);
  }

  watch(id: string, callback: (game: GameRecord) => void): Unsubscribe {
    if (!this.listeners.has(id)) {
      this.listeners.set(id, new Set());
    }

    this.listeners.get(id)!.add(callback);

    return () => {
      const callbacks = this.listeners.get(id);
      if (callbacks) {
        callbacks.delete(callback);
        if (callbacks.size === 0) {
          this.listeners.delete(id);
        }
      }
    };
  }

  async isHealthy(): Promise<boolean> {
    try {
      await this.pool.query('SELECT 1');
      return true;
    } catch {
      return false;
    }
  }

  async exportAll(): Promise<GameRecord[]> {
    const result = await this.pool.query('SELECT * FROM games ORDER BY created_at DESC');
    return result.rows.map((row: any) => this.rowToRecord(row));
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
    const result = await this.pool.query('DELETE FROM games');
    this.listeners.clear();
    return result.rowCount;
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
      synced: row.synced,
      createdAt: row.created_at?.toISOString(),
      updatedAt: row.updated_at?.toISOString(),
    };
  }

  private generateId(): string {
    return `game_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  async close(): Promise<void> {
    if (this.client) {
      await this.client.release();
    }
    await this.pool.end();
  }
}
