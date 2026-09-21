// The WebAssembly the loader uses, declared here because TypeScript ships these types only with its DOM library,
// which a Node package must not load. Node 24 provides the runtime.

declare namespace WebAssembly {
  class Memory {
    readonly buffer: ArrayBuffer
  }
  class Module {
    constructor(bytes: Uint8Array | ArrayBuffer)
  }
  class Instance {
    constructor(module: Module, imports?: object)
    readonly exports: Record<string, unknown>
  }
  class RuntimeError extends Error {}
}
