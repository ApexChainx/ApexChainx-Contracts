declare module '@jest/globals' {
  export function describe(name: string, fn: () => void): void;
  export function it(name: string, fn: () => void): void;
  export function test(name: string, fn: () => void): void;
  export function beforeAll(fn: () => void): void;
  export function afterAll(fn: () => void): void;
  export function beforeEach(fn: () => void): void;
  export function afterEach(fn: () => void): void;
  export function expect(actual?: any): any;
  export const jest: any;
}
