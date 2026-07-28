// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — @hyperchess/core
// File: packages/core/src/board/hfen.ts
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

import { Board } from '../types/board';

/** Export board to (legacy, identity-less) HFEN notation */
export function getBoardHfen(board: Board): string {
  // Piece placement. Standard FEN convention: highest rank first, so we walk
  // rank indices 11 → 0 (matching parsePiecePlacement's reversed reading) —
  // within each rank, files still run 0 → 11 (ascending square index).
  const rankSegments: string[] = [];

  for (let rank = 11; rank >= 0; rank--) {
    let segment = '';
    let emptyCount = 0;

    for (let file = 0; file < 12; file++) {
      const piece = board.pieces[rank * 12 + file];
      if (!piece) {
        emptyCount++;
      } else {
        if (emptyCount > 0) {
          segment += emptyCount;
          emptyCount = 0;
        }
        segment += piece.color === 'white' ? piece.type : piece.type.toLowerCase();
      }
    }

    if (emptyCount > 0) segment += emptyCount;
    rankSegments.push(segment);
  }

  let hfen = rankSegments.join('/');

  // Side to move
  hfen += ` ${board.toMove === 'white' ? 'w' : 'b'}`;

  // Castling rights
  const castling =
    (board.castlingRights.whiteKingSide ? 'K' : '') +
    (board.castlingRights.whiteQueenSide ? 'Q' : '') +
    (board.castlingRights.blackKingSide ? 'k' : '') +
    (board.castlingRights.blackQueenSide ? 'q' : '');
  hfen += ` ${castling || '-'}`;

  // En passant
  if (board.enPassantSquare === -1) {
    hfen += ' -';
  } else {
    const col = String.fromCharCode('a'.charCodeAt(0) + (board.enPassantSquare % 12));
    const row = Math.floor(board.enPassantSquare / 12) + 1;
    hfen += ` ${col}${row}`;
  }

  // Halfmove clock
  hfen += ` ${board.halfmoveClock}`;

  // Fullmove number
  hfen += ` ${board.fullmoveNumber}`;

  return hfen;
}
