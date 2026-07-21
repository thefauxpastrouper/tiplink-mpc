/* @ts-self-types="./tiplink_client_wasm.d.ts" */
import * as wasm from "./tiplink_client_wasm_bg.wasm";
import { __wbg_set_wasm } from "./tiplink_client_wasm_bg.js";

__wbg_set_wasm(wasm);
wasm.__wbindgen_start();
export {
    ClientMpcContext, NonceContext
} from "./tiplink_client_wasm_bg.js";
