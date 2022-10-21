import { RustModule } from "../deps/rust_module";
import { initMadoModule } from "../module/mangadex";


export async function module__Ok__Close() {
    let allmodule = initMadoModule();
    let module = new RustModule(allmodule[0]);
    return await module.close();
}

export function module__Err_ModuleLoadError__MustBeObject() {
    return RustModule.fromRust({} as any);
}
