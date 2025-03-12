<script>
    import { onMount } from "svelte";
    import {
        getQuestion,
        getItemsSorted,
        createQuestion,
        updateQuestion,
    } from "@lib/api";

    let question = {};
    let categories = [];
    let loading = true;
    let error = null;
    let isNew = false;
    let errorMessage = "";
    let formLoading = false;

    onMount(async () => {
        try {
            // Load categories first
            categories = await getItemsSorted("categories");

            const urlParams = new URLSearchParams(window.location.search);
            const questionId = urlParams.get("id");

            if (!questionId) {
                isNew = true;
                loading = false;
                return;
            }

            question = await getQuestion(questionId);
            loading = false;
        } catch (err) {
            error = err.message || "Failed to load question or categories";
            loading = false;
        }
    });

    async function handleSubmit(event) {
        event.preventDefault();
        formLoading = true;
        errorMessage = "";

        try {
            const form = event.target;
            const formData = new FormData(form);
            const questionData = Object.fromEntries(formData.entries());

            Object.keys(questionData).forEach((key) => {
                if (questionData[key] === "") {
                    questionData[key] = null;
                }
            });

            if (Object.keys(question).length > 0 && !isNew) {
                await updateQuestion(questionData);
            } else {
                await createQuestion(questionData);
            }

            window.location.href = "/questions";
        } catch (error) {
            errorMessage = error.message || "An unknown error occurred";
            formLoading = false;
        }
    }
</script>

{#if loading}
    <div class="flex justify-center items-center p-12">
        <div
            class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue"
        ></div>
    </div>
{:else if error}
    <div class="w-2xl mx-auto bg-red/20 p-6 rounded-lg shadow-md text-center">
        <p class="text-red font-bold">Error</p>
        <p>{error}</p>
        <a
            href="/questions"
            class="mt-4 inline-block px-4 py-2 bg-blue/50 text-text rounded-md hover:bg-blue transition-colors"
        >
            Back to Questions
        </a>
    </div>
{:else}
    <form
        id="primaryForm"
        class="mx-auto bg-crust p-6 rounded-lg shadow-md"
        on:submit={handleSubmit}
    >
        {#if !isNew}
            <div class="mb-4">
                <label for="id" class="block text-sm font-medium text-text mb-1"
                    >Question ID</label
                >
                <input
                    type="text"
                    id="id"
                    name="id"
                    value={question.id || ""}
                    readonly
                    class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm cursor-not-allowed"
                />
            </div>
        {/if}

        <div class="mb-4">
            <label for="title" class="block text-sm font-medium text-text mb-1"
                >Title</label
            >
            <input
                type="text"
                id="title"
                name="title"
                value={question.title || ""}
                required
                class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
            />
        </div>

        <div class="mb-4">
            <label for="slug" class="block text-sm font-medium text-text mb-1"
                >Slug</label
            >
            <input
                type="text"
                id="slug"
                name="slug"
                value={question.slug || ""}
                required
                class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
            />
        </div>

        <div class="mb-4">
            <label
                for="description"
                class="block text-sm font-medium text-text mb-1"
                >Description</label
            >
            <textarea
                id="description"
                name="description"
                rows="3"
                class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
                >{question.description || ""}</textarea
            >
        </div>

        <div class="mb-4">
            <label
                for="category_slug"
                class="block text-sm font-medium text-text mb-1">Category</label
            >
            <select
                id="category_slug"
                name="category_slug"
                class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
            >
                <option value=""></option>
                {#each categories as category}
                    <option
                        value={category.slug}
                        selected={question.category_slug === category.slug}
                    >
                        {category.name}
                    </option>
                {/each}
            </select>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="mb-4">
                <label
                    for="start_date_override"
                    class="block text-sm font-medium text-text mb-1"
                    >Start Date Override</label
                >
                <input
                    type="datetime-local"
                    id="start_date_override"
                    name="start_date_override"
                    value={question.start_date_override || ""}
                    class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
                />
            </div>

            <div class="mb-4">
                <label
                    for="end_date_override"
                    class="block text-sm font-medium text-text mb-1"
                    >End Date Override</label
                >
                <input
                    type="datetime-local"
                    id="end_date_override"
                    name="end_date_override"
                    value={question.end_date_override || ""}
                    class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
                />
            </div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div class="mb-4">
                <label
                    for="total_traders"
                    class="block text-sm font-medium text-text mb-1"
                    >Total Traders</label
                >
                <input
                    type="number"
                    id="total_traders"
                    name="total_traders"
                    value={question.total_traders || ""}
                    class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
                />
            </div>

            <div class="mb-4">
                <label
                    for="total_volume"
                    class="block text-sm font-medium text-text mb-1"
                    >Total Volume</label
                >
                <input
                    type="number"
                    id="total_volume"
                    name="total_volume"
                    step="0.01"
                    value={question.total_volume || ""}
                    class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
                />
            </div>

            <div class="mb-4">
                <label
                    for="total_duration"
                    class="block text-sm font-medium text-text mb-1"
                    >Total Duration</label
                >
                <input
                    type="number"
                    id="total_duration"
                    name="total_duration"
                    value={question.total_duration || ""}
                    class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
                />
            </div>
        </div>

        {#if errorMessage}
            <div
                class="max-w-full mx-auto mb-4 p-4 bg-red/20 border border-red text-red rounded-lg"
            >
                <p class="font-medium">Error:</p>
                <p>{errorMessage}</p>
            </div>
        {/if}

        <div class="flex justify-end space-x-4 mt-6">
            <a
                href="/questions"
                class="px-4 py-2 bg-blue/50 text-text rounded-md hover:bg-blue transition-colors"
            >
                Cancel
            </a>
            <button
                type="submit"
                disabled={formLoading}
                class="px-4 py-2 bg-green/50 text-white rounded-md hover:bg-green transition-colors"
            >
                {formLoading
                    ? "Saving..."
                    : Object.keys(question).length > 0 && !isNew
                      ? "Update"
                      : "Create"} Question
            </button>
        </div>
    </form>
{/if}
