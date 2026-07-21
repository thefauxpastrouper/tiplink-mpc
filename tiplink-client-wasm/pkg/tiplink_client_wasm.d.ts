/* tslint:disable */
/* eslint-disable */

export class ClientMpcContext {
    free(): void;
    [Symbol.dispose](): void;
    combine_signatures(s_client_bytes: Uint8Array, s_server_bytes: Uint8Array, r_client_bytes: Uint8Array, r_server_bytes: Uint8Array): Uint8Array;
    compute_nonce_commitment(): NonceContext;
    compute_partial_signature(k_client_bytes: Uint8Array, r_server_bytes: Uint8Array, r_client_bytes: Uint8Array, combined_pubkey_bytes: Uint8Array, message: Uint8Array): Uint8Array;
    static from_bytes(bytes: Uint8Array): ClientMpcContext;
    get_public_point(): Uint8Array;
    get_share(): Uint8Array;
    constructor();
}

export class NonceContext {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    get_k(): Uint8Array;
    get_r(): Uint8Array;
}
