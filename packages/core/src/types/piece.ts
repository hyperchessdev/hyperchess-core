// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — @hyperchess/core
// File: packages/core/src/types/piece.ts
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

import { Color } from './move';

/** Piece type (side-agnostic) */
export type PieceType = 'P' | 'N' | 'B' | 'R' | 'Q' | 'K' | 'E' | 'H';

/** Represents a piece on the board */
export interface Piece {
  type: PieceType;
  color: Color;
}

/** Piece position on board */
export interface PiecePlacement {
  piece: Piece;
  square: number; // 0-143
}

/** Piece symbols for display */
export const PIECE_SYMBOLS = {
  white: {
    P: '♙',
    N: '♘',
    B: '♗',
    R: '♖',
    Q: '♕',
    K: '♔',
    E: '🦅',
    H: '🦅',
  },
  black: {
    P: '♟',
    N: '♞',
    B: '♝',
    R: '♜',
    Q: '♛',
    K: '♚',
    E: '🦅',
    H: '🦅',
  },
};
