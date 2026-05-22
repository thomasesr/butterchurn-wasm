/* tslint:disable */
/* eslint-disable */

export class Visualizer {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * JS calls this with an AudioNode — stored as opaque JsValue.
     * Actual audio routing is done via setAudioData each frame.
     */
    connect_audio(_gain_node: any): void;
    destroy(): void;
    load_extra_images(image_data: any): void;
    load_preset(preset: any, blend_time: number): void;
    constructor(canvas: HTMLCanvasElement, opts: any);
    /**
     * Render one frame. `now` is performance.now() in milliseconds.
     */
    render(now: number): void;
    set_audio_data(time_domain: Float32Array, frequency: Float32Array): void;
    set_renderer_size(width: number, height: number): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_visualizer_free: (a: number, b: number) => void;
    readonly visualizer_connect_audio: (a: number, b: any) => void;
    readonly visualizer_destroy: (a: number) => void;
    readonly visualizer_load_extra_images: (a: number, b: any) => [number, number];
    readonly visualizer_load_preset: (a: number, b: any, c: number) => [number, number];
    readonly visualizer_new: (a: any, b: any) => [number, number, number];
    readonly visualizer_render: (a: number, b: number) => void;
    readonly visualizer_set_audio_data: (a: number, b: any, c: any) => void;
    readonly visualizer_set_renderer_size: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
