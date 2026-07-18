import { Board, HfenString } from '@hyperchess/core';
import { Move } from '@hyperchess/core';

/** Theme name or custom theme object */
export type Theme = 'classic' | 'modern' | 'dark' | 'highcontrast' | string;

/** Board interaction mode */
export type InteractionMode = 'view' | 'play' | 'analyze';

/** All board component props */
export interface BoardProps {
  /** Initial board state (HFEN or Board object) */
  hfen?: HfenString | Board;

  /** Theme name or custom theme */
  theme?: Theme;

  /** Callback when move is made */
  onMove?: (move: Move) => void;

  /** Callback when square is selected */
  onSquareClick?: (square: number) => void;

  /** Enable drag-and-drop moves */
  enableDragDrop?: boolean;

  /** Show rank/file coordinates */
  showCoordinates?: boolean;

  /** Flip board (black on bottom) */
  flipBoard?: boolean;

  /** Highlight last move */
  highlightLastMove?: boolean;

  /** Highlight legal moves for selected piece */
  highlightLegalMoves?: boolean;

  /** Interaction mode */
  mode?: InteractionMode;

  /** Disable user interaction */
  disabled?: boolean;

  /** Custom CSS class */
  className?: string;

  /** Piece style (unicode, svg, or font) */
  pieceStyle?: 'unicode' | 'svg' | 'font';

  /** Animation speed (ms for move animation) */
  animationSpeed?: number;

  /** Board size in pixels (or responsive) */
  size?: number | 'responsive';
}

/** Selection state for a piece */
export interface SelectionState {
  selectedSquare: number | null;
  legalMoves: number[];
  lastMove: { from: number; to: number } | null;
}

/** Interaction handler signatures */
export interface BoardHandlers {
  onMove: (move: Move) => void;
  onSquareClick: (square: number) => void;
  onDragStart: (square: number) => void;
  onDragEnd: (square: number) => void;
  onDrop: (from: number, to: number) => void;
}
