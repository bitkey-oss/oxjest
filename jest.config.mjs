/** @type {import('jest').Config} */
const config = {
  transform: {
    "\\.ts$": "oxjest",
  },
  extensionsToTreatAsEsm: [".ts"],
};

export default config;
