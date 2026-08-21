export type ExpectedSemanticAction = {
  startMs: number;
  endMs: number;
  action: "CUT" | "KEEP" | "HIGHLIGHT";
  taxonomy: "false_start" | "repeated_take" | "redundant_sentence" | "important_statement" | "none";
};

export type SemanticFixtureCase = {
  description: string;
  segments: Array<{ id: string; startMs: number; endMs: number; text: string }>;
  expected: ExpectedSemanticAction[];
};

export type SemanticEvaluation = {
  cases: number;
  passed: number;
  failed: number;
  accuracy: number;
  failures: string[];
};

export function evaluateSemanticFixture(
  fixtures: SemanticFixtureCase[],
  outputs: Array<Array<Partial<ExpectedSemanticAction>>>,
): SemanticEvaluation {
  const failures: string[] = [];
  let passed = 0;
  fixtures.forEach((fixture, index) => {
    const actual = outputs[index] ?? [];
    const expected = new Map(
      fixture.expected.map((item) => [`${item.startMs}:${item.endMs}`, item]),
    );
    const observed = new Map(actual.map((item) => [`${item.startMs}:${item.endMs}`, item]));
    const ok =
      expected.size === observed.size &&
      [...expected].every(([key, item]) => {
        const candidate = observed.get(key);
        return candidate?.action === item.action && candidate?.taxonomy === item.taxonomy;
      });
    if (ok) passed += 1;
    else failures.push(fixture.description);
  });
  const cases = fixtures.length;
  return {
    cases,
    passed,
    failed: cases - passed,
    accuracy: cases === 0 ? 0 : passed / cases,
    failures,
  };
}
