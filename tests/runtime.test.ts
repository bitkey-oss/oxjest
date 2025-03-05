import { jest } from "@jest/globals";
import { createMockFactory } from "oxjest/runtime";

describe("createMockFactory", () => {
  it("create a mock for function", async () => {
    const factory = createMockFactory({
      greet(): string {
        return "Hello, world!";
      },
    });

    const mock = factory();

    expect(jest.isMockFunction(mock.greet)).toBe(true);
  });

  it("create a mock for class", async () => {
    class Greeter {
      static greet(): string {
        return "Hello, world!";
      }
    }

    const factory = createMockFactory({ Greeter });
    const mock = factory();

    expect(jest.isMockFunction(mock.Greeter.greet)).toBe(true);
  });
});
