<script setup lang="ts">
import { useNotificationStore } from "../stores/notifications";
import NotificationCard from "./NotificationCard.vue";
const notifications = useNotificationStore();
</script>
<template>
  <aside class="notification-viewport" aria-label="Notifications">
    <TransitionGroup name="notification">
      <NotificationCard
        v-for="notice in notifications.notices"
        :key="notice.id"
        :notice="notice"
        @dismiss="notifications.dismiss"
      />
    </TransitionGroup>
  </aside>
</template>
