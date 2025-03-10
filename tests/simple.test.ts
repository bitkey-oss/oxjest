import { greet } from "./greeter.ts";

jest.mock("./greeter.ts");

describe("Simple", () => {
  it("mock a module", () => {
    jest.mocked(greet).mockReturnValueOnce("Hello from mocked module!");

    expect(greet()).toBe("Hello from mocked module!");
  });
});
