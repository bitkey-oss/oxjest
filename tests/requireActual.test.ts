jest.mock("./greeter", () => ({
  greet: () => "Hello from mocked module!",
}));

describe("requireActual", () => {
  it("mock a module", () => {
    const greeter = jest.requireActual("./greeter");

    expect(greeter.greet()).toBe("Hello, world!");
  });
});
