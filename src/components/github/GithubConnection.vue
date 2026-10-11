<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref, shallowRef } from "vue";
import {
  call,
  asError,
  type GithubStatus,
  type GithubVerification,
} from "../../lib/api";
import { useRepositoryStore } from "../../stores/repository";
import { useNotificationStore } from "../../stores/notifications";
const store = useRepositoryStore(),
  notifications = useNotificationStore();
const status = shallowRef<GithubStatus>(),
  verification = shallowRef<GithubVerification>(),
  error = ref(""),
  busy = ref(false),
  privateAccess = ref(false);
let version = 0;
async function refresh() {
  try {
    status.value = await call<GithubStatus>("github_status");
  } catch (e) {
    error.value = asError(e).message;
  }
}
async function cancel() {
  ++version;
  if (busy.value) await store.attempt(() => call("github_cancel"));
  busy.value = false;
  verification.value = undefined;
}
async function connect() {
  const current = ++version;
  busy.value = true;
  error.value = "";
  try {
    const result = await call<GithubVerification>("github_start", {
      private: privateAccess.value,
    });
    if (current !== version) return;
    verification.value = result;
    await store.attempt(() =>
      call("github_open", { repository: null, number: null }),
    );
    await call("github_finish", { requestId: result.id });
    if (current !== version) return;
    verification.value = undefined;
    await refresh();
    notifications.notify({
      kind: "success",
      title: "GitHub connected",
      message: "Authorization saved in your OS credential vault.",
    });
  } catch (e) {
    if (current === version) {
      error.value = asError(e).message;
      store.fail(e);
    }
  } finally {
    if (current === version) busy.value = false;
  }
}
async function disconnect() {
  busy.value = true;
  try {
    await call("github_disconnect");
    await refresh();
    notifications.notify({
      kind: "info",
      title: "GitHub disconnected",
      message:
        "Local authorization removed. Revoke the OAuth app on GitHub to invalidate its token.",
    });
  } catch (e) {
    error.value = asError(e).message;
    store.fail(e);
  } finally {
    busy.value = false;
  }
}
onMounted(refresh);
onBeforeUnmount(() => {
  void cancel();
});
</script>
<template>
  <section class="github-connection">
    <h3>GitHub connection</h3>
    <p class="muted">
      Optional access to pull requests. Git credentials remain managed by your
      existing Git configuration.
    </p>
    <p v-if="status && !status.configured" role="status">
      This build needs a GitHub OAuth application client ID with Device Flow
      enabled. See docs/GITHUB.md. Public pull requests remain available
      anonymously.
    </p>
    <p v-if="status?.connected">
      Connected · credentials stored in Keychain or Credential Manager.
    </p>
    <label v-if="!status?.connected"
      ><input v-model="privateAccess" type="checkbox" :disabled="busy" />
      Authorize private repositories</label
    >
    <p v-if="privateAccess && !status?.connected" class="muted">
      GitHub's OAuth repo scope grants broad repository access, including write
      capabilities. Git Toys uses this connection only for reading pull
      requests; you can revoke it in GitHub Settings → Applications.
    </p>
    <div class="feature-toolbar">
      <button
        v-if="!status?.connected"
        class="button"
        :disabled="!status?.configured || busy"
        @click="connect"
      >
        {{ busy ? "Waiting for GitHub…" : "Sign in with GitHub" }}</button
      ><button v-else class="button" :disabled="busy" @click="disconnect">
        Disconnect GitHub</button
      ><button v-if="busy && verification" class="button" @click="cancel">
        Cancel sign-in
      </button>
    </div>
    <div v-if="verification" role="status">
      <p>
        Enter
        <strong class="device-code">{{ verification.userCode }}</strong> at
        github.com/login/device. Expires in
        {{ Math.ceil(verification.expiresIn / 60) }} minutes.
      </p>
      <button
        class="button"
        @click="
          store.attempt(() =>
            call('github_open', { repository: null, number: null }),
          )
        "
      >
        Open GitHub verification
      </button>
    </div>
    <p v-if="error" role="alert">{{ error }}</p>
  </section>
</template>
