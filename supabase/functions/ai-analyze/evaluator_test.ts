import { assertEquals } from "jsr:@std/assert@1";
import { evaluateSemanticFixture, type SemanticFixtureCase } from "./evaluator.ts";

Deno.test("semantic fixture evaluator detects exact canonical outputs", () => {
  const fixture: SemanticFixtureCase = {
    description: "range-boundary",
    segments: [{ id: "s1", startMs: 0, endMs: 1000, text: "Xin chào" }],
    expected: [{ startMs: 0, endMs: 1000, action: "KEEP", taxonomy: "none" }],
  };
  assertEquals(
    evaluateSemanticFixture(
      [fixture],
      [[{ startMs: 0, endMs: 1000, action: "KEEP", taxonomy: "none" }]],
    ),
    { cases: 1, passed: 1, failed: 0, accuracy: 1, failures: [] },
  );
});
