// ==========================================================================
// hopSolverWorker.js
// ==========================================================================
// Worker "feuille" du pool HOP : charge une seule fois le module WASM puis
// résout, à la demande, les mondes déterminisés qu'on lui confie.
//
// Message reçu : { type: "solve-batch", requestId, knownJson, seeds: number[], budget }
// Message renvoyé, un par lot : { type: "batch-result", requestId, results: WorldOutcome[] }
// En cas d'erreur : { type: "batch-error", requestId, message }

let wasmReady = null;

async function ensureWasmLoaded() {
  if (!wasmReady) {
    wasmReady = import("../wasm/solitaire_solver.js").then(async (mod) => {
      await mod.default();
      mod.wasm_init_panic_hook();
      return mod;
    });
  }
  return wasmReady;
}

self.onmessage = async (event) => {
  const { type, requestId, knownJson, seeds, budget } = event.data;
  if (type !== "solve-batch") return;

  try {
    const mod = await ensureWasmLoaded();
    const results = seeds.map((seed) => {
      const json = mod.wasm_solve_one_world(knownJson, BigInt(seed), BigInt(budget));
      const parsed = JSON.parse(json);
      if (parsed.error) throw new Error(parsed.error);
      return parsed;
    });
    self.postMessage({ type: "batch-result", requestId, results });
  } catch (err) {
    self.postMessage({
      type: "batch-error",
      requestId,
      message: (err && err.message) || "Erreur inattendue pendant la résolution d'un lot de mondes.",
    });
  }
};
