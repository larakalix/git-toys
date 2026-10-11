import { ref, shallowRef, onBeforeUnmount, watch } from "vue";
import {
  call,
  asError,
  onEvent,
  type PreviewRequest,
  type PreviewResult,
} from "../lib/api";
import { useRepositoryStore } from "../stores/repository";
export function useGitPreview() {
  const store = useRepositoryStore(),
    result = shallowRef<PreviewResult>(),
    loading = ref(false),
    error = ref(""),
    progress = ref("");
  let id = "",
    version = 0,
    stateToken = "";
  async function cancel() {
    ++version;
    loading.value = false;
    if (id) {
      const current = id;
      id = "";
      await store.attempt(() => call("preview_cancel", { requestId: current }));
    }
  }
  async function run(request: PreviewRequest) {
    await cancel();
    const current = ++version;
    const repository = store.activeId;
    id = crypto.randomUUID();
    const requestId = id;
    stateToken = store.snapshot?.stateToken ?? "";
    result.value = undefined;
    loading.value = true;
    error.value = "";
    progress.value = "Preparing isolated preview…";
    const unlisten = await onEvent<{ id: string; message: string }>(
      "preview-progress",
      (event) => {
        if (event.id === id) progress.value = event.message;
      },
    );
    try {
      const response = await call<PreviewResult>("preview_operation", {
        id: repository,
        requestId,
        request,
      });
      if (current === version && repository === store.activeId) {
        if (store.snapshot?.stateToken !== stateToken)
          error.value =
            "Repository changed during preview. Generate a new preview.";
        else result.value = response;
      }
    } catch (e) {
      if (current === version) {
        error.value = asError(e).message;
        store.fail(e);
      }
    } finally {
      unlisten();
      if (current === version) {
        id = "";
        loading.value = false;
      }
    }
  }
  function invalidate() {
    result.value = undefined;
    void cancel();
  }
  watch(() => store.activeId, invalidate);
  watch(
    () => store.snapshot?.stateToken,
    (token) => {
      if (result.value && token !== stateToken) {
        result.value = undefined;
        error.value = "Repository changed. Generate a new preview.";
      }
    },
  );
  onBeforeUnmount(() => {
    void cancel();
  });
  return {
    result,
    loading,
    error,
    progress,
    run,
    cancel,
    invalidate,
    expected: () => stateToken,
  };
}
