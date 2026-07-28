// Export types
export type { GameStore } from './types/game-store';
export { isStoreOnline, syncStores } from './types/game-store';
export type { GameRecord, GameQueryOptions, Unsubscribe } from './types/game-record';
export { GameNotFoundError, ValidationError, SyncError } from './types/game-record';

// Export adapters
export { MemoryGameStore } from './adapters/memory';
export { PostgresGameStore } from './adapters/postgres';
export { SqliteGameStore } from './adapters/sqlite';
export { FirebaseGameStore } from './adapters/firebase';
export { SupabaseGameStore } from './adapters/supabase';

// Default export is memory store for development
export { MemoryGameStore as default } from './adapters/memory';
