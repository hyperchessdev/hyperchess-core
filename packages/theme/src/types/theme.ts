/** A complete theme definition */
export interface Theme {
  name: string;

  /** Light square color */
  lightSquare: string;

  /** Dark square color */
  darkSquare: string;

  /** Highlight color for last move */
  highlightLastMove: string;

  /** Highlight color for legal moves */
  highlightLegalMoves: string;

  /** Highlight color for selected piece */
  highlightSelected: string;

  /** Piece rendering style */
  pieces: {
    style: 'unicode' | 'svg' | 'font';
    font?: string;
  };

  /** Check highlight color */
  checkHighlight?: string;

  /** Text color for coordinates */
  coordinateText?: string;
}

/** All built-in theme names */
export type BuiltInTheme = 'classic' | 'modern' | 'dark' | 'highcontrast' | 'pastel';

/** Theme configuration for CSS generation */
export interface ThemeCssConfig {
  lightSquare: string;
  darkSquare: string;
  highlightColor: string;
  selectedColor: string;
}
