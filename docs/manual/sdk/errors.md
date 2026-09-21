# Errors

Every error `evoke` raises is data with a fix attached. Its message is the CLI's own line:
`lights: vocabulary "rooms" is empty  →  evoke vocab rooms add <word> "<meaning>"`.

```ts
import { DiagnosticError, EvokeError, FailureError, FaultError } from "@evoke-build/evoke"
```

| Class             | `kind`         | Means                                                       | CLI exit |
| :---------------- | :------------- | :---------------------------------------------------------- | :------- |
| `DiagnosticError` | `"diagnostic"` | Something a person fixes in the files or the environment: `problems`, each `{ reflex?, at?, message, fix, command }` | 3 |
| `FaultError`      | `"fault"`      | The adapter failed, or its answers did not validate: `fault`, one of transport, status, retired, unanswered, malformed, unrecorded | 4 |
| `FailureError`    | `"failure"`    | A body or the machine failed: `what`, `why`, `fix`          | 1        |

All three extend `EvokeError`, which carries `command`. That is the literal line that fixes it. It is empty when
the only fix is to try again. `fix` is the structured value, and `command` is its rendering. Render nothing
yourself.

```ts
try {
  await project.handle(input, handlers)
} catch (error) {
  if (error instanceof DiagnosticError) for (const p of error.problems) log(`${p.message}  →  ${p.command}`)
  else if (error instanceof FaultError) retryLater(error.fault)
  else if (error instanceof FailureError) log(error.message)
  else throw error
}
```

- **Misuse is a plain `TypeError`.** That means running a decision that is not `run` or `confirm`, running a
  confirm without `{ confirmed: true }`, or handing a decision to a project other than the one that made it.
- **Aborts reject with the signal's own reason**, never wrapped.
- **A bug in `evoke`**, a trap in the core, is a plain `Error` naming it.

**Next:** the [Reference](../reference/cli.md).
