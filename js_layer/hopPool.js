// ==========================================================================
// hopPool.js
// ==========================================================================
// Répartit les mondes entre plusieurs hopSolverWorker.js (un par cœur
// logique) et agrège les votes à mesure que les résultats arrivent.
//
// Usage :
//   import { runHopAnalysis } from "./hopPool.js";
//   const result = await runHopAnalysis({
//     known, numWorlds: 2000, budgetPerWorld: 200_000,
//     onProgress: (partial) => setHopPartial(partial),
//   });

const DEFAULT_MAX_WORKERS = 8;

function poolSize() {
  const cores = (typeof navigator !== "undefined" && navigator.hardwareConcurrency) || 4;
  return Math.max(1, Math.min(DEFAULT_MAX_WORKERS, cores));
}

function splitSeeds(numWorlds, n, baseSeed) {
  const batches = Array.from({ length: n }, () => []);
  for (let i = 0; i < numWorlds; i++) {
    batches[i % n].push(baseSeed + i);
  }
  return batches.filter((b) => b.length > 0);
}

function moveKey(move) {
  return JSON.stringify(move);
}

export function runHopAnalysis({ known, numWorlds, budgetPerWorld, baseSeed = 1, onProgress, signal }) {
  return new Promise((resolve, reject) => {
    const n = poolSize();
    const seedBatches = splitSeeds(numWorlds, n, baseSeed);
    const knownJson = JSON.stringify(known);

    const votes = new Map();
    let worldsCompleted = 0;
    let worldsSolved = 0;
    let unsolvedFoundationSum = 0;
    let unsolvedCount = 0;
    let batchesPending = seedBatches.length;
    let settled = false;

    const workers = seedBatches.map(() => new Worker(new URL("./hopSolverWorker.js", import.meta.url), { type: "module" }));

    function currentPartial() {
      const ranked = [...votes.values()]
        .sort((a, b) => b.votes - a.votes)
        .map((v) => ({ move: v.move, votes: v.votes, voteShare: worldsSolved > 0 ? v.votes / worldsSolved : 0 }));
      return {
        worldsCompleted,
        worldsSolved,
        ranked,
        avgFoundationWhenUnsolved: unsolvedCount > 0 ? unsolvedFoundationSum / unsolvedCount : 0,
      };
    }

    function cleanup() {
      workers.forEach((w) => w.terminate());
    }

    function finishOnce(fn) {
      if (settled) return;
      settled = true;
      cleanup();
      fn();
    }

    if (signal) {
      signal.addEventListener("abort", () => finishOnce(() => reject(new DOMException("Analyse HOP annulée", "AbortError"))), { once: true });
    }

    if (seedBatches.length === 0) {
      finishOnce(() => resolve(currentPartial()));
      return;
    }

    workers.forEach((worker, i) => {
      worker.onmessage = (event) => {
        const { type, results, message } = event.data;
        if (type === "batch-result") {
          for (const r of results) {
            worldsCompleted++;
            if (r.solved && r.first_move) {
              worldsSolved++;
              const key = moveKey(r.first_move);
              const entry = votes.get(key) || { move: r.first_move, votes: 0 };
              entry.votes++;
              votes.set(key, entry);
            } else {
              unsolvedCount++;
              unsolvedFoundationSum += r.best_foundation_count || 0;
            }
          }
          onProgress?.(currentPartial());
          batchesPending--;
          if (batchesPending === 0) {
            finishOnce(() => resolve(currentPartial()));
          }
        } else if (type === "batch-error") {
          finishOnce(() => reject(new Error(message)));
        }
      };
      worker.onerror = (err) => {
        finishOnce(() => reject(new Error(`hopSolverWorker #${i} en erreur: ${err.message}`)));
      };
      worker.postMessage({
        type: "solve-batch",
        requestId: i,
        knownJson,
        seeds: seedBatches[i],
        budget: budgetPerWorld,
      });
    });
  });
}
