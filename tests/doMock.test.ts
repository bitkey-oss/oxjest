describe("doMock", () => {
  beforeEach(() => {
    jest.resetModules();
  });

  it("mock a module first", async () => {
    jest.doMock("./greeter.ts", () => {
      return {
        greet: jest.fn(() => "Hello from first mocked module!"),
      };
    });
    const { greet } = await import("./greeter.ts");

    expect(greet()).toBe("Hello from first mocked module!");
  });

  it("mock a module second", async () => {
    jest.doMock("./greeter.ts", () => {
      return {
        greet: jest.fn(() => "Hello from second mocked module!"),
      };
    });
    const { greet } = await import("./greeter.ts");

    expect(greet()).toBe("Hello from second mocked module!");
  });
});
