import { ref, computed } from 'vue';
import {
  Board as BoardType,
  Move,
  HfenString,
  createBoard,
  applyMove,
  generateLegalMoves,
} from '@hyperchess/core';
import type { SelectionState } from '../types/props';

/**
 * Vue 3 composable for managing board state
 */
export function useBoardState(initialHfen?: HfenString | BoardType) {
  // `ref()` takes a value, not a lazy-initializer function like React's
  // `useState` — compute the initial board up front instead.
  const initialBoard: BoardType =
    !initialHfen ? createBoard() : typeof initialHfen === 'string' ? createBoard(initialHfen) : initialHfen;
  const board = ref<BoardType | null>(initialBoard);

  const selection = ref<SelectionState>({
    selectedSquare: null,
    legalMoves: [],
    lastMove: null,
  });

  const legalMoves = computed(() => {
    if (!board.value) return [];
    return generateLegalMoves(board.value);
  });

  const selectSquare = (square: number) => {
    if (!board.value) return;

    const piece = board.value.pieces[square];

    if (piece && piece.color === board.value.toMove) {
      const moves = legalMoves.value.filter((m) => m.from === square).map((m) => m.to);

      selection.value = {
        ...selection.value,
        selectedSquare: square,
        legalMoves: moves,
      };
    } else if (
      selection.value.selectedSquare !== null &&
      selection.value.legalMoves.includes(square)
    ) {
      makeMove({ from: selection.value.selectedSquare, to: square });
    } else {
      selection.value = {
        ...selection.value,
        selectedSquare: null,
        legalMoves: [],
      };
    }
  };

  const makeMove = (move: Move) => {
    if (!board.value) return false;

    const isLegal = legalMoves.value.some((m) => m.from === move.from && m.to === move.to);
    if (!isLegal) return false;

    board.value = applyMove(board.value, move);

    selection.value = {
      selectedSquare: null,
      legalMoves: [],
      lastMove: { from: move.from, to: move.to },
    };

    return true;
  };

  const reset = () => {
    board.value = createBoard();
    selection.value = {
      selectedSquare: null,
      legalMoves: [],
      lastMove: null,
    };
  };

  return {
    board,
    selection,
    selectSquare,
    makeMove,
    reset,
    legalMoves,
  };
}
