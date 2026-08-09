//! Outil en ligne de commande : résout une donne Klondike aléatoire (ou une
//! graine donnée) et affiche la solution trouvée.

use solitaire_solver::dealer::random_deal;
use solitaire_solver::solve;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let seed: u64 = args
        .get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64);

    let budget: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2_000_000);

    println!("Solveur de solitaire — graine {seed}, budget {budget} nœuds");
    println!("Résolution en cours...");

    let start = std::time::Instant::now();
    let state = random_deal(seed);
    let out = solve(state, budget);
    let elapsed = start.elapsed();

    println!();
    if out.solved {
        println!("✅ GAGNABLE — solution en {} coups, trouvée en {:.2}s ({} nœuds explorés)", out.moves.len(), elapsed.as_secs_f64(), out.nodes_explored);
        for (i, mv) in out.moves.iter().enumerate() {
            println!("  {:>3}. {:?}", i + 1, mv);
        }
    } else if out.budget_reached {
        println!(
            "❓ NON DÉTERMINÉ — budget de {} nœuds épuisé sans conclure ({:.2}s, meilleure progression: {}/52 cartes en fondation)",
            budget,
            elapsed.as_secs_f64(),
            out.best_foundation_count
        );
    } else {
        println!(
            "❌ PROUVÉ NON-GAGNABLE — {} nœuds explorés en {:.2}s, meilleure progression atteinte: {}/52 cartes en fondation",
            out.nodes_explored,
            elapsed.as_secs_f64(),
            out.best_foundation_count
        );
    }
}
