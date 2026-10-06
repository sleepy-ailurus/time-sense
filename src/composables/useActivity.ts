import { ref } from "vue";
import {
  getTodayStats,
  getTodayTotal,
  getCurrentActivity,
} from "../api";
import type { AppStat, TodayTotal, CurrentActivity } from "../api/types";

const todayStats = ref<AppStat[]>([]);
const todayTotal = ref<TodayTotal>({
  totalSeconds: 0,
  activeSeconds: 0,
  idleSeconds: 0,
});
const currentActivity = ref<CurrentActivity | null>(null);
const loading = ref(false);

export function useActivity() {
  async function fetchTodayStats() {
    try {
      const [stats, total] = await Promise.all([
        getTodayStats(),
        getTodayTotal(),
      ]);
      todayStats.value = stats;
      todayTotal.value = total;
    } catch (err) {
      console.error("Failed to fetch today stats:", err);
    }
  }

  async function fetchCurrentActivity() {
    try {
      const activity = await getCurrentActivity();
      currentActivity.value = activity;
    } catch (err) {
      console.error("Failed to fetch current activity:", err);
    }
  }

  return {
    todayStats,
    todayTotal,
    currentActivity,
    loading,
    fetchTodayStats,
    fetchCurrentActivity,
  };
}
