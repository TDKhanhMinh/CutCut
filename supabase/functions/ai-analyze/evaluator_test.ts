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

Deno.test("Vietnamese fixture corpus preserves bounded conservative taxonomy", async () => {
  const fixtureUrl = new URL("./fixtures/semantic_eval_vi.json", import.meta.url);
  const fixtures = JSON.parse(await Deno.readTextFile(fixtureUrl)) as SemanticFixtureCase[];
  const evaluation = evaluateSemanticFixture(
    fixtures,
    fixtures.map((fixture) => fixture.expected),
  );

  assertEquals(evaluation, {
    cases: 5,
    passed: 5,
    failed: 0,
    accuracy: 1,
    failures: [],
  });
  assertEquals(
    fixtures.find((fixture) => fixture.description.startsWith("Intentional repetition"))?.expected,
    [
      { startMs: 40000, endMs: 42000, action: "KEEP", taxonomy: "none" },
      { startMs: 42000, endMs: 44000, action: "KEEP", taxonomy: "none" },
    ],
  );
});
