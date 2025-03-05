export function greet(): string {
  return "Hello, world!";
}

export class Greeter {
  static greetStatic(): string {
    return "Hello from static method!";
  }

  greet(): string {
    return "Hello from class method!";
  }
}
