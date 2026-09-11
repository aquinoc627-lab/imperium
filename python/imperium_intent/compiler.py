"""Intent Compiler - NL to IR."""

from typing import Any

from imperium_intent.models import IntentIR


class IntentCompiler:
    """Compiles natural language to Intent IR."""

    async def compile(self, nl: str, context: dict[str, Any]) -> IntentIR:
        del nl, context
        raise NotImplementedError(
            "LLM compilation is Phase 3. Use the rules compiler in Phase 1."
        )
