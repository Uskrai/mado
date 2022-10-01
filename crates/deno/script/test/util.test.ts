import { Ok } from "../deps/error";
import { RustModule } from "../deps/rust_module";
import { initMadoModule } from "../module/mangadex";


export async function module__Ok__Close() {
    let module = new RustModule(initMadoModule()[0]);
    let it = await module.close();
    return it;
}

// export function module__Err_RequestError__MustBeObject() {
//     // let module = new RustModule({});
// }
