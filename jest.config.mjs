/** @type {import('jest').Config} */
const config = {
  rootDir: "./tests",
  transform: {
    "\\.ts$": "oxjest",
  },
  extensionsToTreatAsEsm: [".ts"],
};

export default config;
