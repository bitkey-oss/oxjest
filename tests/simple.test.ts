import { greet } from "./greeter";

jest.mock("./greeter");

describe("Simple", () => {
  it("mock a module", () => {
    jest.mocked(greet).mockReturnValueOnce("Hello from mocked module!");

    expect(greet()).toBe("Hello from mocked module!");
  });
});
