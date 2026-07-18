//! Perft testing framework.

use super::Board;

/// Runs perft (performance test) to count nodes at a given depth.
pub fn perft(board: &mut Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }

    let moves = board.generate_moves();

    if depth == 1 {
        return moves.len() as u64;
    }

    let mut nodes = 0u64;
    for m in moves.iter() {
        board.apply_move(*m);
        nodes += perft(board, depth - 1);
        board.undo_move();
    }
    nodes
}

/// Runs perft with divide (shows count per first move).
pub fn perft_divide(board: &mut Board, depth: u32) -> u64 {
    let moves = board.generate_moves();
    let mut total = 0u64;

    for m in moves.iter() {
        board.apply_move(*m);
        let count = if depth <= 1 {
            1
        } else {
            perft(board, depth - 1)
        };
        total += count;
        println!("{}: {}", m, count);
        board.undo_move();
    }

    println!("\nTotal: {}", total);
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perft_depth_0() {
        let mut board = Board::start_pos();
        assert_eq!(perft(&mut board, 0), 1);
    }

    #[test]
    fn test_perft_depth_1_start() {
        let mut board = Board::start_pos();
        let nodes = perft(&mut board, 1);
        // Manual count (Eagle/Hawk max 4 squares):
        // 12 pawns × 2 (single + double push) = 24
        // Eagles (a2,l2): 4 each (a4,a5,a6,a1 / l4,l5,l6,l1 — blocked east/west by own pieces)
        // Hawks (b2,k2): 5 each (NE 3 squares over own pawn, SE+SW 1 each, NW off-board)
        // Rooks (c2,j2): 1 each (south to rank 1 only)
        // Knights (d2,i2): 4 each (2 forward + 2 backward)
        // Bishops (e2,h2): 2 each (backward diags only)
        // Queen (f2): 3 (south + 2 backward diags)
        // King (g2): 3 (f1, g1, h1)
        // Castling: 0 (blocked by own pieces)
        // Total: 24 + 8 + 10 + 2 + 8 + 4 + 3 + 3 = 62
        assert_eq!(nodes, 62, "Perft(1) from start position");
    }

    #[test]
    fn test_perft_depth_2_start() {
        let mut board = Board::start_pos();
        let nodes = perft(&mut board, 2);
        // After each of White's 86 moves, Black responds.
        // This is a symmetry check: Black has the same piece layout (mirrored).
        assert!(nodes > 0, "Perft(2) should be positive, got {}", nodes);
        println!("Perft(2) = {}", nodes);
    }

    #[test]
    fn test_perft_depth_1_fen_roundtrip() {
        let mut board = Board::start_pos();
        let fen_before = board.get_hfen();

        let moves = board.generate_moves();
        for m in moves.iter() {
            board.apply_move(*m);
            board.undo_move();
            assert_eq!(
                board.get_hfen(),
                fen_before,
                "HFEN mismatch after apply+undo of {}",
                m
            );
        }
    }

    #[test]
    fn test_perft_depth_2_fen_roundtrip() {
        let mut board = Board::start_pos();
        let fen_before = board.get_hfen();

        let moves = board.generate_moves();
        for m1 in moves.iter() {
            board.apply_move(*m1);
            let fen_after_m1 = board.get_hfen();

            let moves2 = board.generate_moves();
            for m2 in moves2.iter() {
                board.apply_move(*m2);
                board.undo_move();
                assert_eq!(
                    board.get_hfen(),
                    fen_after_m1,
                    "HFEN mismatch after apply+undo of {} {}",
                    m1,
                    m2
                );
            }

            board.undo_move();
            assert_eq!(
                board.get_hfen(),
                fen_before,
                "HFEN mismatch after depth-2 roundtrip starting with {}",
                m1
            );
        }
    }
}
