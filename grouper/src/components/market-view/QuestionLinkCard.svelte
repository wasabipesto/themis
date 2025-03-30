<script lang="ts">
  import type { Question } from "@types";

  export let question: Question | null = null;
</script>

{#if question}
  <div class="bg-crust p-6 rounded-lg shadow-md mb-6">
    <h2 class="text-xl font-semibold mb-2">Question Link</h2>
    <div class="p-2">
      <p>
        This market is linked to question:
        <a
          href={`/questions/edit?id=${question.id}`}
          class="text-blue hover:underline"
        >
          {question.title}
        </a>
      </p>
    </div>
  </div>
{/if}
