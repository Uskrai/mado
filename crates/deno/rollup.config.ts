import { nodeResolve } from '@rollup/plugin-node-resolve';
import ts from "rollup-plugin-typescript2";
import common from "@rollup/plugin-commonjs";
import multiInput from "rollup-plugin-multi-input";
import nodePolyfills from 'rollup-plugin-polyfill-node';


export default {
        input: ['script/deps/*.ts', 'script/module/*.ts'],
        output: {
            dir: 'dist',
            format: 'es',
            chunkFileNames: 'chunk/[name].js', 
        },
        watch: {
            chokidar: {
                usePolling: true
            }
        },
        plugins: [
            multiInput({
                relative: 'script',
            }),
            nodePolyfills(),
            nodeResolve({browser: true}),
            ts(),
            common(),
            // rename({
            //     ["**/*.js", "**/*.ts"],
            // })
        ]
}
