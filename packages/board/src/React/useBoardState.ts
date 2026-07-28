import { useState, useCallback, useMemo } from 'react';
import { Board as BoardType, Move, HfenString, createBoard, applyMove, generateLegalMoves } from '@hyperchess/core';
import { SelectionState } from '../types/props';

/**
 * Custom hook for managing board state
 * Handles piece selection, move validation, and board updates
 */
export function useBoardState(initialHfen?: HfenString | BoardType) {
  const [board, setBoard] = useState<BoardType | null>(() => {
    if (!initialHfen) {
      return createBoard();
    }
    if (typeof initialHfen === 'string') {
      return createBoard(initialHfen);
    }
    return initialHfen;
  });

  const [selection, setSelection] = useState<SelectionState>({
    selectedSquare: null,
    legalMoves: [],
    lastMove: null,
  });

  const legalMoves = useMemo(() => {
    if (!board) return [];
    return generateLegalMoves(board);
  }, [board]);

  const selectSquare = useCallback(
    (square: number) => {
      if (!board) return;

      const piece = board.pieces[square];

      // Click on own piece: select it
      if (piece && piece.color === board.toMove) {
        const moves = legalMoves
          .filter((m) => m.from === square)
          .map((m) => m.to);

        setSelection({
          ...selection,
          selectedSquare: square,
          legalMoves: moves,
        });
      }
      // Click on destination: try move
      else if (selection.selectedSquare !== null && selection.legalMoves.includes(square)) {
        makeMove({ from: selection.selectedSquare, to: square });
      }
      // Click elsewhere: deselect
      else {
        setSelection({
          ...selection,
          selectedSquare: null,
          legalMoves: [],
        });
      }
    },
    [board, selection, legalMoves]
  );

  const makeMove = useCallback(
    (move: Move) => {
      if (!board) return false;

      const isLegal = legalMoves.some((m) => m.from === move.from && m.to === move.to);
      if (!isLegal) return false;

      const newBoard = applyMove(board, move);
      setBoard(newBoard);

      setSelection({
        selectedSquare: null,
        legalMoves: [],
        lastMove: { from: move.from, to: move.to },
      });

      return true;
    },
    [board, legalMoves]
  );

  const reset = useCallback(() => {
    setBoard(createBoard());
    setSelection({
      selectedSquare: null,
      legalMoves: [],
      lastMove: null,
    });
  }, []);

  return {
    board,
    selection,
    selectSquare,
    makeMove,
    reset,
    legalMoves,
  };
}
