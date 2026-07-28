import type { Theme, BuiltInTheme } from './types/theme';

/** Classic chess board theme */
export const THEME_CLASSIC: Theme = {
  name: 'classic',
  lightSquare: '#f0d9b5',
  darkSquare: '#baca44',
  highlightLastMove: 'rgba(184, 202, 68, 0.5)',
  highlightLegalMoves: 'rgba(184, 202, 68, 0.3)',
  highlightSelected: 'rgba(184, 202, 68, 0.7)',
  pieces: {
    style: 'unicode',
  },
  coordinateText: '#999',
};

/** Modern minimalist theme */
export const THEME_MODERN: Theme = {
  name: 'modern',
  lightSquare: '#ffffff',
  darkSquare: '#e0e0e0',
  highlightLastMove: 'rgba(100, 150, 200, 0.4)',
  highlightLegalMoves: 'rgba(100, 150, 200, 0.2)',
  highlightSelected: 'rgba(100, 150, 200, 0.6)',
  pieces: {
    style: 'unicode',
  },
  coordinateText: '#666',
};

/** Dark theme */
export const THEME_DARK: Theme = {
  name: 'dark',
  lightSquare: '#2c2c2c',
  darkSquare: '#1a1a1a',
  highlightLastMove: 'rgba(255, 200, 0, 0.4)',
  highlightLegalMoves: 'rgba(255, 200, 0, 0.2)',
  highlightSelected: 'rgba(255, 200, 0, 0.6)',
  pieces: {
    style: 'unicode',
  },
  coordinateText: '#999',
};

/** High contrast theme for accessibility */
export const THEME_HIGHCONTRAST: Theme = {
  name: 'highcontrast',
  lightSquare: '#ffffff',
  darkSquare: '#000000',
  highlightLastMove: 'rgba(255, 0, 0, 0.5)',
  highlightLegalMoves: 'rgba(0, 255, 0, 0.3)',
  highlightSelected: 'rgba(0, 0, 255, 0.6)',
  pieces: {
    style: 'unicode',
  },
  coordinateText: '#000000',
};

/** Soft pastel theme */
export const THEME_PASTEL: Theme = {
  name: 'pastel',
  lightSquare: '#fce4ec',
  darkSquare: '#f8bbd0',
  highlightLastMove: 'rgba(244, 67, 54, 0.3)',
  highlightLegalMoves: 'rgba(244, 67, 54, 0.2)',
  highlightSelected: 'rgba(244, 67, 54, 0.5)',
  pieces: {
    style: 'unicode',
  },
  coordinateText: '#c2185b',
};

/** Map of all built-in themes */
const THEMES: Record<BuiltInTheme, Theme> = {
  classic: THEME_CLASSIC,
  modern: THEME_MODERN,
  dark: THEME_DARK,
  highcontrast: THEME_HIGHCONTRAST,
  pastel: THEME_PASTEL,
};

/** Get a built-in theme by name */
export function getTheme(name: BuiltInTheme): Theme {
  return THEMES[name];
}

/** Apply theme to DOM element via CSS variables */
export function applyTheme(element: HTMLElement, theme: Theme): void {
  element.style.setProperty('--hc-light-square', theme.lightSquare);
  element.style.setProperty('--hc-dark-square', theme.darkSquare);
  element.style.setProperty('--hc-highlight-last-move', theme.highlightLastMove);
  element.style.setProperty('--hc-highlight-legal-moves', theme.highlightLegalMoves);
  element.style.setProperty('--hc-highlight-selected', theme.highlightSelected);
  if (theme.coordinateText) {
    element.style.setProperty('--hc-coordinate-text', theme.coordinateText);
  }
}

export type { Theme, BuiltInTheme };
