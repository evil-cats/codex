The active thread goal objective was edited by the user.

The exact objective in <thread_goal_context> supersedes every previous thread goal objective. Treat that context as the authoritative current goal state for this thread.

Budget:
- Tokens used: {{ tokens_used }}
- Token budget: {{ token_budget }}
- Tokens remaining: {{ remaining_tokens }}

Adjust the current turn to pursue the updated objective. Avoid continuing work that only served the previous objective unless it also helps the updated objective.

Use update_goal only under the completion, blocked-audit, or cancellation conditions in <thread_goal_context>. Cancellation is immediate and does not require the blocked audit.
