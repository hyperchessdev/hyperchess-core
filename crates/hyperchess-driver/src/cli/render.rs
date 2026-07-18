//! Board PNG rendering using the hyperchess_v2.ttf custom font.

use hyperchess_rules::board::Board;
use hyperchess_rules::core::sq::SQ;
use hyperchess_rules::core::Piece;

use ab_glyph::{FontRef, PxScale};
use image::{Rgb, RgbImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut};
use imageproc::rect::Rect;

const SQUARE_SIZE: u32 = 60;
const BORDER: u32 = 30;
const BOARD_SIZE: u32 = 12;
const IMG_SIZE: u32 = BOARD_SIZE * SQUARE_SIZE + 2 * BORDER;

const LIGHT_SQ: Rgb<u8> = Rgb([240, 217, 181]);
const DARK_SQ: Rgb<u8> = Rgb([181, 136, 99]);
const TEXT_COLOR: Rgb<u8> = Rgb([0, 0, 0]);
const BG_COLOR: Rgb<u8> = Rgb([48, 48, 48]);
const LABEL_COLOR: Rgb<u8> = Rgb([200, 200, 200]);

/// Font glyph mappings for pieces.
fn piece_glyph(piece: Piece) -> Option<char> {
    match piece {
        Piece::WhiteKing => Some('k'),
        Piece::BlackKing => Some('l'),
        Piece::WhiteQueen => Some('q'),
        Piece::BlackQueen => Some('w'),
        Piece::WhiteRook => Some('r'),
        Piece::BlackRook => Some('t'),
        Piece::WhiteBishop => Some('b'),
        Piece::BlackBishop => Some('n'),
        Piece::WhiteKnight => Some('h'),
        Piece::BlackKnight => Some('j'),
        Piece::WhitePawn => Some('p'),
        Piece::BlackPawn => Some('o'),
        Piece::WhiteEagle => Some('v'),
        Piece::BlackEagle => Some('u'),
        Piece::WhiteHawk => Some('f'),
        Piece::BlackHawk => Some('e'),
        Piece::None => None,
    }
}

/// Save the board as a PNG image.
pub fn save_png(board: &Board, path: &str) {
    let font_data = include_bytes!("../../assets/hyperchess_v2.ttf");
    let font = match FontRef::try_from_slice(font_data) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to load font: {}. Skipping PNG render.", e);
            return;
        }
    };

    // Also load a system-like font for labels
    let label_scale = PxScale { x: 14.0, y: 14.0 };
    let piece_scale = PxScale { x: 48.0, y: 48.0 };

    let mut img = RgbImage::from_pixel(IMG_SIZE, IMG_SIZE, BG_COLOR);

    // Draw board squares
    for rank in 0..12u32 {
        for file in 0..12u32 {
            let x = BORDER + file * SQUARE_SIZE;
            let y = BORDER + (11 - rank) * SQUARE_SIZE;
            let color = if (rank + file) % 2 == 0 {
                DARK_SQ
            } else {
                LIGHT_SQ
            };
            draw_filled_rect_mut(
                &mut img,
                Rect::at(x as i32, y as i32).of_size(SQUARE_SIZE, SQUARE_SIZE),
                color,
            );

            // Draw piece
            let sq = SQ::make(file as u8, rank as u8);
            let piece = board.piece_at(sq);
            if let Some(glyph_char) = piece_glyph(piece) {
                let glyph_str = glyph_char.to_string();
                let px = x + 6;
                let py = y + 4;
                draw_text_mut(
                    &mut img,
                    TEXT_COLOR,
                    px as i32,
                    py as i32,
                    piece_scale,
                    &font,
                    &glyph_str,
                );
            }
        }
    }

    // Draw file labels (a-l) at bottom
    let file_labels = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l'];
    for (i, &label) in file_labels.iter().enumerate() {
        let x = BORDER + i as u32 * SQUARE_SIZE + SQUARE_SIZE / 2 - 4;
        let y = BORDER + 12 * SQUARE_SIZE + 5;
        draw_text_mut(
            &mut img,
            LABEL_COLOR,
            x as i32,
            y as i32,
            label_scale,
            &font,
            &label.to_string(),
        );
    }

    // Draw rank labels (1-12) on the left
    for rank in 0..12u32 {
        let x = 5;
        let y = BORDER + (11 - rank) * SQUARE_SIZE + SQUARE_SIZE / 2 - 7;
        let label = format!("{}", rank + 1);
        draw_text_mut(
            &mut img,
            LABEL_COLOR,
            x,
            y as i32,
            label_scale,
            &font,
            &label,
        );
    }

    match img.save(path) {
        Ok(_) => println!("PNG saved to {}", path),
        Err(e) => eprintln!("Error saving PNG: {}", e),
    }
}
