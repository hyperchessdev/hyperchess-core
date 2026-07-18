import { GameStore } from '../types/game-store';
import { GameRecord, GameQueryOptions, GameNotFoundError } from '../types/game-record';

/**
 * In-memory game store for testing and development
 * All data is lost when process exits!
 */
export class MemoryGameStore implements GameStore {
  private games: Map<string, GameRecord> = new Map();
  private watchers: Map<string, Set<(game: GameRecord) => void>> = new Map();

  async saveGame(game: GameRecord): Promise<string> {
    // Generate ID if not provided
    const id = game.id || this.generateId();
    const recordWithId = {
      ...game,
      id,
      updatedAt: new Date().toISOString(),
    };

    this.games.set(id, recordWithId);

    // Notify watchers
    this.notifyWatchers(id, recordWithId);

    return id;
  }

  async loadGame(id: string): Promise<GameRecord> {
    const game = this.games.get(id);
    if (!game) {
      throw new GameNotFoundError(id);
    }
    return structuredClone(game); // Deep copy to prevent mutations
  }

  async deleteGame(id: string): Promise<void> {
    if (!this.games.has(id)) {
      throw new GameNotFoundError(id);
    }
    this.games.delete(id);

    // Notify watchers
    const watchers = this.watchers.get(id);
    if (watchers) {
      this.watchers.delete(id);
    }
  }

  async listGames(options?: GameQueryOptions): Promise<GameRecord[]> {
    let games = Array.from(this.games.values());

    // Filter by userId if provided
    if (options?.userId) {
      games = games.filter((g) => g.userId === options.userId);
    }

    // Filter by result if provided
    if (options?.result) {
      games = games.filter((g) => g.result === options.result);
    }

    // Sort by date
    const sortOrder = options?.sort === 'desc' ? -1 : 1;
    games.sort((a, b) => {
      const dateA = new Date(a.createdAt || 0).getTime();
      const dateB = new Date(b.createdAt || 0).getTime();
      return sortOrder * (dateA - dateB);
    });

    // Apply pagination
    const offset = options?.offset || 0;
    const limit = options?.limit || games.length;
    games = games.slice(offset, offset + limit);

    return games.map((g) => structuredClone(g));
  }

  async countGames(options?: GameQueryOptions): Promise<number> {
    let games = Array.from(this.games.values());

    if (options?.userId) {
      games = games.filter((g) => g.userId === options.userId);
    }

    if (options?.result) {
      games = games.filter((g) => g.result === options.result);
    }

    return games.length;
  }

  watch(id: string, callback: (game: GameRecord) => void): () => void {
    if (!this.watchers.has(id)) {
      this.watchers.set(id, new Set());
    }

    this.watchers.get(id)!.add(callback);

    // Return unsubscribe function
    return () => {
      const watchers = this.watchers.get(id);
      if (watchers) {
        watchers.delete(callback);
        if (watchers.size === 0) {
          this.watchers.delete(id);
        }
      }
    };
  }

  async isHealthy(): Promise<boolean> {
    return true; // In-memory store is always healthy
  }

  async exportAll(): Promise<GameRecord[]> {
    return Array.from(this.games.values()).map((g) => structuredClone(g));
  }

  async importGames(games: GameRecord[]): Promise<number> {
    for (const game of games) {
      await this.saveGame(game);
    }
    return games.length;
  }

  async clear(): Promise<number> {
    const count = this.games.size;
    this.games.clear();
    this.watchers.clear();
    return count;
  }

  private notifyWatchers(id: string, game: GameRecord): void {
    const watchers = this.watchers.get(id);
    if (watchers) {
      watchers.forEach((callback) => callback(structuredClone(game)));
    }
  }

  private generateId(): string {
    return `game_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }
}
