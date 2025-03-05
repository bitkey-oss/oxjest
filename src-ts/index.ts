import createCacheKeyFunction from "@jest/create-cache-key-function";
import type { SyncTransformer, TransformedSource, TransformerFactory } from "@jest/transform";

import { transform } from "../sys/index.js";

import packageJson from "../package.json";

const oxjestVersion = packageJson.version;
const dumpCodeEnabled = !!process.env.OXJEST_DUMP_CODE;

const factory: TransformerFactory<SyncTransformer> = {
  createTransformer(): SyncTransformer {
    const getCacheKey = createCacheKeyFunction(
      ["package.json", "tsconfig.json"],
      [oxjestVersion, dumpCodeEnabled ? crypto.randomUUID() : ""],
    );

    return {
      canInstrument: false,
      getCacheKey: getCacheKey as SyncTransformer["getCacheKey"],
      process(sourceText, sourcePath): TransformedSource {
        const { code, map } = transform(sourceText, sourcePath);
        if (dumpCodeEnabled) {
          console.debug(sourcePath, code);
        }

        return { code, map };
      },
    };
  },
};

export default factory;
