# oxjest

Advanced native ESM support for Jest, built on top of Oxc.

> [!NOTE]
> This is not an officially supported product of Bitkey, Inc.

## Why?

Though Jest already supports ESM (ECMAScript modules), there are many features which are not supported in ESM yet.
For example, the global `jest` object, hoisted mocks, and more.
oxjest polyfills such features by **code transformation**.
You can reuse your tests **without** fully rewriting for ESM.

oxjest is built on top of [Oxc](https://oxc.rs/), a high-performance toolkit for ECMAScript and TypeScript syntaxes.
Your tests can adopt ESM without increasing testing time, even if they're written in TypeScript!

## Installation

```shell
npm install -D oxjest
```

oxjest is a Jest Transformer.
Add `oxjest` as a transformer for `.js` files in your `jest.config.js` to enable.
See [Code Transformation](https://jestjs.io/docs/code-transformation) for details.

```js
/** @type {import("jest").Config} */
const config = {
  transform: {
    "\\.js": "oxjest",
  },
};

export default config;
```

## Features

### Built-in TypeScript Transpiling

oxjest transpiles TypeScript files to ECMAScript, powered by [oxc_transformer](https://github.com/oxc-project/oxc/tree/main/crates/oxc_transformer).
To enable TypeScript support, use the following Jest configuration:

```js
/** @type {import("jest").Config} */
const config = {
  transform: {
    "\\.ts": "oxjest",
  },
};

export default config;
```

> [!NOTE]
> oxjest does **NOT** check any types in your code.
> Run `tsc` separately to ensure your code is valid in TypeScript.

### Jest Object Injecting

Previously, the `jest` object is available as globals.
In ESM, it is required to import it from `@jest/globals` module or to reference `import.meta.jest`.
oxjest transforms the global `jest` references to `import.meta.jest` automatically.

#### Before

```js
jest.fn();
```

#### After

```js
import.meta.jest.fn();
```

### Mock Hoisting

oxjest hoists `jest.unstable_mockModule()` calls to the top of the module.
Plus, all imports after mocking are turned into await imports.
This ensures the mocking is evaluated before the mocked module is imported.

`jest.mock` calls are converted to `jest.unstable_mockModule` calls for compatibility.

> [!TIP]
> In ESM, module imports are evaluated statically; they are evaluated before any code within the module,
> even the code is appeared before imports. This is why turning into dynamic imports is required.

#### Before

```js
import { greet } from "./greeter.js";

jest.mock("./greeter.js", () => ({
  greet: jest.fn(),
}));
```

#### After

```js
jest.unstable_mockModule("./greeter.js", () => ({
  greet: jest.fn(),
}));

const __oxjest_import_0__ = await import("./greeter.js"),
      greet = __oxjest_import_0__.greet;
```

### Auto Mocking

While Jest does generate mocks automatically in CommonJS, it is not available in ESM yet.
oxjest injects the runtime module to generate mocks from the evaluated actual module.

> [!TIP]
> Auto mocking requires to evaluate the actual module (and its submodules) to get the module exports.
> Using manual mocking is recommended for larger modules.

#### Before

```js
jest.mock("./greeter.js");
```

#### After

```js
import * as __oxjest__ from "oxjest/runtime";

jest.unstable_mockModule("./greeter.js", __oxjest__.createMockFactory(await import("./greeter.js")));
```

### `jest.requireActual` Support

When the module is already mocked, the actual module can't be retrieved by `import`.
Jest has `jest.requireActual` for this, but it's not available in ESM yet.
oxjest turns them into dynamic imports and hoists to the top of the module.

#### Before

```js
jest.unstable_mockModule("./greeter.js", () => ({
  greet: jest.fn(),
}));

const greeter = jest.requireActual("./greeter.js");
```

#### After

```js
const __oxjest_actual_0__ = await import("./greeter.js");

jest.unstable_mockModule("./greeter.js", () => ({
  greet: jest.fn(),
}));

const greeter = __oxjest_actual_0__;
```

## Caveats

### Default Export Problem

Consider the following module is to be mocked by `jest.mock`:

```js
export default function greet() {
  return "Hello, world!";
}
```

Previously, default exports could be mocked in the following form:

```js
jest.mock("./greeter.js", () => jest.fn());
```

This is no longer available in ESM. To mock the default export, you can use `default` key in the factory:

```js
jest.mock("./greeter.js", () => ({
  default: jest.fn(),
}));
```

### Where is `__dirname` and `__filename`?

Though `import.meta.dirname` and `import.meta.filename` are available for the alternative, they're not supported in
Jest yet (will be available in v30). To use them, upgrade to `jest@alpha` or use the following snippet:

```js
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
```

> [!WARNING]
> Note that the value of `import.meta` depends on where it's wrote.
> Do not try setting `globalThis.__dirname` or `globalThis.__filename` or you will get the wrong value at runtime.
