import { writeFile } from "fs/promises";
import lcovSourcemap from "lcov-sourcemap-ts";

let ignore = /node_modules|xmldom.js/;

let [,, lcov, sourceDir, output] = process.argv;
console.log(lcov, sourceDir);

let outputFile = await lcovSourcemap
  .getTransformedFiles({
    lcov,
    sourceDir,
  })
  .then((it) =>
    it.filter((it) => {
      return !ignore.test(it.path);
    })
  );

let string = await lcovSourcemap.getOutputLcov(outputFile, sourceDir);

await writeFile(output, string);
