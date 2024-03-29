import common from "@rollup/plugin-commonjs";
import { nodeResolve } from "@rollup/plugin-node-resolve";
import multiInputPkg from "rollup-plugin-multi-input";
const multiInput = multiInputPkg.default;
import ts from "rollup-plugin-typescript2";

export default {
  input: ["script/deps/*.ts", "script/module/*.ts", "script/test/*.ts"],
  output: {
    dir: "dist",
    format: "es",
    // chunkFileNames: "chunk/[name].js",
    sourcemap: 'both',
  },
  watch: {
    chokidar: {
      usePolling: true,
    },
  },
  plugins: [
    multiInput({ relative: "script/" }),
    // nodePolyfills(),
    nodeResolve({ browser: true }),
    ts(),
    common(),
    // rename({
    //     ["**/*.js", "**/*.ts"],
    // })
  ],
};
