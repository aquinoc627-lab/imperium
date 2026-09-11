/**
 * Contract: browser-api re-exports stay stable for the workbench.
 */
import { describe, it, expect } from "vitest";
import * as api from "./browser-api";

describe("browser-api surface", () => {
  it("exports compileRules and constants", () => {
    expect(typeof api.compileRules).toBe("function");
    expect(api.COMPILER_VERSION).toBeTruthy();
    expect(api.ECHO_CAPABILITY).toBe("cap.echo");
  });

  it("compiles a canonical echo", () => {
    const r = api.compileRules("Echo this message: ping");
    expect(r.ok).toBe(true);
    if (r.ok) {
      expect(r.ir.capability).toBe(api.ECHO_CAPABILITY);
    }
  });

  it("exports localPropose", () => {
    expect(typeof api.localPropose).toBe("function");
  });
});
