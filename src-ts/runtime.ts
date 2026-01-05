import { jest } from "@jest/globals";

type MockMetadataType = "object" | "array" | "regexp" | "function" | "constant" | "collection" | "null" | "undefined";

type MockMetadata<T, MetadataType = MockMetadataType> = {
  ref?: number;
  members?: Record<string, MockMetadata<T>>;
  mockImpl?: T;
  name?: string;
  refID?: number;
  type?: MetadataType;
  value?: T;
  length?: number;
};

// https://github.com/jestjs/jest/blob/93ca9d3cf6db4b88393ab02b8d0007bcdf30e4bb/packages/jest-mock/src/index.ts#L416C1-L419C1
function getObjectType(value: unknown): string {
  return Object.prototype.toString.apply(value).slice(8, -1);
}

// https://github.com/jestjs/jest/blob/93ca9d3cf6db4b88393ab02b8d0007bcdf30e4bb/packages/jest-mock/src/index.ts#L420C1-L455C2
function getType(ref?: unknown): MockMetadataType | null {
  const typeName = getObjectType(ref);
  if (
    typeName === "Function" ||
    typeName === "AsyncFunction" ||
    typeName === "GeneratorFunction" ||
    typeName === "AsyncGeneratorFunction"
  ) {
    return "function";
  }
  if (Array.isArray(ref)) {
    return "array";
  }
  if (typeName === "Object" || typeName === "Module") {
    return "object";
  }
  if (typeName === "Number" || typeName === "String" || typeName === "Boolean" || typeName === "Symbol") {
    return "constant";
  }
  if (typeName === "Map" || typeName === "WeakMap" || typeName === "Set") {
    return "collection";
  }
  if (typeName === "RegExp") {
    return "regexp";
  }
  if (ref === undefined) {
    return "undefined";
  }
  if (ref === null) {
    return "null";
  }
  return null;
}

// https://github.com/jestjs/jest/blob/93ca9d3cf6db4b88393ab02b8d0007bcdf30e4bb/packages/jest-mock/src/index.ts#L457
function isReadonlyProp(object: unknown, prop: string): boolean {
  if (prop === "arguments" || prop === "caller" || prop === "callee" || prop === "name" || prop === "length") {
    const typeName = getObjectType(object);
    return (
      typeName === "Function" ||
      typeName === "AsyncFunction" ||
      typeName === "GeneratorFunction" ||
      typeName === "AsyncGeneratorFunction"
    );
  }

  if (prop === "source" || prop === "global" || prop === "ignoreCase" || prop === "multiline") {
    return getObjectType(object) === "RegExp";
  }

  return false;
}

// https://github.com/jestjs/jest/blob/93ca9d3cf6db4b88393ab02b8d0007bcdf30e4bb/packages/jest-mock/src/index.ts#L506C1-L548C4
function getSlots(object?: Record<string, any>): Array<string> {
  if (!object) {
    return [];
  }

  const slots = new Set<string>();

  const ObjectProto = Object.prototype;
  const FunctionProto = Function.prototype;
  const RegExpProto = RegExp.prototype;

  while (object != null && object !== ObjectProto && object !== FunctionProto && object !== RegExpProto) {
    const ownNames = Object.getOwnPropertyNames(object);

    for (const prop of ownNames) {
      if (!isReadonlyProp(object, prop)) {
        const propDesc = Object.getOwnPropertyDescriptor(object, prop);
        if ((propDesc !== undefined && !propDesc.get) || object.__esModule) {
          slots.add(prop);
        }
      }
    }

    object = Object.getPrototypeOf(object);
  }

  return [...slots];
}

function getMetadata<T = unknown>(component: T, _refs?: Map<T, number>): MockMetadata<T> | null {
  const refs = _refs || new Map<T, number>();
  const ref = refs.get(component);
  if (ref != null) {
    return { ref };
  }

  const type = getType(component);
  if (!type) {
    return null;
  }

  const metadata: MockMetadata<T> = { type };
  if (type === "constant" || type === "collection" || type === "undefined" || type === "null") {
    metadata.value = component;
    return metadata;
  }
  if (type === "function") {
    // @ts-expect-error component is a function so it has a name, but not
    // necessarily a string: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Function/name#function_names_in_classes
    const componentName = component.name;
    if (typeof componentName === "string") {
      metadata.name = componentName;
    }
    if (jest.isMockFunction(component)) {
      metadata.mockImpl = component.getMockImplementation() as T;
    }
  }

  metadata.refID = refs.size;
  refs.set(component, metadata.refID);

  let members: Record<string, MockMetadata<T>> | null = null;
  // Leave arrays alone
  if (type !== "array") {
    // @ts-expect-error component is object
    for (const slot of getSlots(component)) {
      if (type === "function" && jest.isMockFunction(component) && slot.startsWith("mock")) {
        continue;
      }
      // @ts-expect-error no index signature
      const slotMetadata = getMetadata<T>(component[slot], refs);
      if (slotMetadata) {
        if (!members) {
          members = {};
        }
        members[slot] = slotMetadata;
      }
    }
  }

  if (members) {
    metadata.members = members;
  }

  return metadata;
}

function getComponent<T>(metadata: MockMetadata<T>): unknown {
  if (metadata.type === "object") {
    return new Object();
  }

  if (metadata.type === "array") {
    return [];
  }

  if (metadata.type === "regexp") {
    return new RegExp("");
  }

  if (
    metadata.type === "constant" ||
    metadata.type === "collection" ||
    metadata.type === "null" ||
    metadata.type === "undefined"
  ) {
    return metadata.value;
  }

  if (metadata.type === "function") {
    return jest.fn();
  }

  throw new Error("unexpected type");
}

// https://github.com/jestjs/jest/blob/93ca9d3cf6db4b88393ab02b8d0007bcdf30e4bb/packages/jest-mock/src/index.ts#L926
function generateMock<T extends object>(metadata: MockMetadata<T>, refs: Record<number, unknown>): jest.Mocked<T> {
  const mock: any = getComponent(metadata);
  if (metadata.refID != null) {
    refs[metadata.refID] = mock;
  }

  for (const slot of getSlots(metadata.members)) {
    const slotMetadata = metadata.members?.[slot] || {};
    if (slotMetadata.ref == null) {
      mock[slot] = generateMock(slotMetadata, refs);
    }
  }

  return mock;
}

export function createMockFactory<T extends Record<string, unknown>>(actual: T): () => jest.Mocked<T> {
  return () => {
    const refs: Record<number, unknown> = {};
    const metadata = getMetadata(actual);
    if (!metadata) {
      throw new Error("could not retrieve metadata from the object");
    }
    return generateMock(metadata, refs) as jest.Mocked<T>;
  };
}
