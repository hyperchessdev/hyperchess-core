/**
 * A complete game record (HPGN + metadata)
 */
export interface GameRecord {
  /** Unique game identifier */
  id: string;

  /** Game in HPGN format */
  hpgn: string;

  /** Starting position (HFEN) */
  hfen: string;

  /** Player names */
  players?: {
    white?: string;
    black?: string;
  };

  /** Game metadata */
  metadata?: {
    event?: string;
    site?: string;
    date?: string;
    round?: string;
    whiteElo?: number;
    blackElo?: number;
    timeControl?: string;
  };

  /** Game result (1-0, 0-1, 1/2-1/2, *) */
  result?: '1-0' | '0-1' | '1/2-1/2' | '*';

  /** User/owner ID (for multi-user systems) */
  userId?: string;

  /** When created (ISO 8601) */
  createdAt?: string;

  /** When last modified (ISO 8601) */
  updatedAt?: string;

  /** Whether synced to cloud (for offline systems) */
  synced?: boolean;
}

/**
 * Query options for listing games
 */
export interface GameQueryOptions {
  /** Filter by user ID */
  userId?: string;

  /** Filter by result */
  result?: '1-0' | '0-1' | '1/2-1/2' | '*';

  /** Maximum results to return */
  limit?: number;

  /** Offset for pagination */
  offset?: number;

  /** Sort order (asc/desc by date) */
  sort?: 'asc' | 'desc';
}

/**
 * Unsubscribe function returned by watch()
 */
export type Unsubscribe = () => void;

/**
 * Error types
 */
export class GameNotFoundError extends Error {
  constructor(id: string) {
    super(`Game not found: ${id}`);
    this.name = 'GameNotFoundError';
  }
}

export class ValidationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ValidationError';
  }
}

export class SyncError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'SyncError';
  }
}
