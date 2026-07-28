import { GameRecord, GameQueryOptions, Unsubscribe } from './game-record';

/**
 * Universal game storage interface
 * Implementations: PostgreSQL, SQLite, Firebase, Supabase, Memory
 *
 * All methods are async for compatibility with remote backends
 */
export interface GameStore {
  /**
   * Save a new game or update existing
   * @param game Game record to save
   * @returns ID of saved game (auto-generated if not provided)
   */
  saveGame(game: GameRecord): Promise<string>;

  /**
   * Load a game by ID
   * @param id Game ID
   * @returns Game record or throws GameNotFoundError
   */
  loadGame(id: string): Promise<GameRecord>;

  /**
   * Delete a game
   * @param id Game ID
   */
  deleteGame(id: string): Promise<void>;

  /**
   * List games (optionally filtered)
   * @param options Query options
   * @returns Array of game records
   */
  listGames(options?: GameQueryOptions): Promise<GameRecord[]>;

  /**
   * Count games (optionally filtered)
   * @param options Query options (omit limit/offset for count)
   * @returns Total count
   */
  countGames(options?: GameQueryOptions): Promise<number>;

  /**
   * Watch a game for real-time updates
   * @param id Game ID
   * @param callback Called when game changes
   * @returns Unsubscribe function
   */
  watch(id: string, callback: (game: GameRecord) => void): Unsubscribe;

  /**
   * Health check - verify backend is accessible
   * @returns true if backend is operational
   */
  isHealthy(): Promise<boolean>;

  /**
   * Export all games (useful for backups)
   * @returns Array of all games
   */
  exportAll?(): Promise<GameRecord[]>;

  /**
   * Import games from backup
   * @param games Array of games to import
   * @returns Number of imported games
   */
  importGames?(games: GameRecord[]): Promise<number>;

  /**
   * Clear all games (dangerous!)
   * @returns Number of games deleted
   */
  clear?(): Promise<number>;
}

/**
 * Check if a game store is online/offline
 */
export async function isStoreOnline(store: GameStore): Promise<boolean> {
  try {
    return await store.isHealthy();
  } catch {
    return false;
  }
}

/**
 * Sync games between two stores (useful for offline→online sync)
 */
export async function syncStores(
  fromStore: GameStore,
  toStore: GameStore,
  options?: { userId?: string; overwrite?: boolean }
): Promise<number> {
  const games = await fromStore.listGames({
    userId: options?.userId,
  });

  let syncedCount = 0;

  for (const game of games) {
    try {
      if (options?.overwrite) {
        // Delete if exists, then save
        try {
          await toStore.deleteGame(game.id);
        } catch {
          // Game doesn't exist in target, that's ok
        }
      }

      await toStore.saveGame(game);
      syncedCount++;
    } catch (error) {
      console.warn(`Failed to sync game ${game.id}:`, error);
    }
  }

  return syncedCount;
}
