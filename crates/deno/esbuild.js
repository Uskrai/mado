import esbuild from "esbuild";
import fastglob from "fast-glob";

esbuild.build({
  entryPoints: [...await fastglob("script/deps/*.ts"), ... await fastglob("script/module/*.ts"), ...await fastglob("script/test/*.ts")],
  format: "esm",
  platform: "browser",
  outdir: "dist",
  bundle: true,
  treeShaking: true,
  sourcemap: "both",
  splitting: true,
  incremental: true,
  watch: {
    onRebuild(error, result) {
      if (error) console.error('watch build failed', error)
      else console.log('watch build succeeded:', result)
    },
  },
  tsconfig: "tsconfig.json",
  chunkNames: "chunk/[name]-[hash]",
  plugins: [
    // nodePolyfills(),
  ]
}).then(_ => {
    console.log("watching");
  });


