use hyperchess_rules::board::Board;
#[test]
fn find_terminal_fens() {
    // Deterministic pseudo-random self-play until terminal; print the HFEN + flags.
    use hyperchess_rules::HyperMove;
    let mut seed: u64 = 0x1234_5678_9abc_def0;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut found = 0;
    for g in 0..400 {
        let mut b = Board::start_pos();
        for _ in 0..400 {
            if b.is_game_over() {
                println!(
                    "TERMINAL g={} over={} result={} hfen={}",
                    g,
                    b.is_game_over(),
                    b.game_result(),
                    b.get_hfen()
                );
                found += 1;
                break;
            }
            let moves = b.generate_moves();
            if moves.is_empty() {
                break;
            }
            let v: Vec<HyperMove> = moves.iter().copied().collect();
            let idx = (next() as usize) % v.len();
            b.apply_move(v[idx]);
        }
        if found >= 3 {
            break;
        }
    }
    assert!(found > 0, "no terminal positions found");
}
