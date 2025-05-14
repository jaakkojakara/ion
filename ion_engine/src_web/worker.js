importScripts("pkg/ion_game.js");
self.onmessage = async function(event) {
  try {
    // Initialize the WASM module with the shared memory
    const wasmModule = await wasm_bindgen("pkg/ion_game_bg.wasm", event.data[0]);

    // Call the entry point with the closure address
    wasmModule.worker_entry_point(Number(event.data[1]));
  } catch (error) {
    console.error("Error in worker:", error);
  } finally {
    self.close();  // Terminate the worker after execution
  }
};
