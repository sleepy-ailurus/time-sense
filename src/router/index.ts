import { createRouter, createWebHashHistory, RouteRecordRaw } from "vue-router";
import DashboardView from "../views/DashboardView.vue";
import CategoryView from "../views/CategoryView.vue";
import RulesView from "../views/RulesView.vue";
import SettingsView from "../views/SettingsView.vue";

// 扩展 vue-router 的 meta 类型
declare module "vue-router" {
  interface RouteMeta {
    title?: string;
    icon?: string;
  }
}

const routes: RouteRecordRaw[] = [
  {
    path: "/",
    name: "dashboard",
    component: DashboardView,
    meta: { title: "概览", icon: "LayoutDashboard" },
  },
  {
    path: "/categories",
    name: "categories",
    component: CategoryView,
    meta: { title: "分类统计", icon: "PieChart" },
  },
  {
    path: "/rules",
    name: "rules",
    component: RulesView,
    meta: { title: "规则管理", icon: "Filter" },
  },
  {
    path: "/settings",
    name: "settings",
    component: SettingsView,
    meta: { title: "设置", icon: "Settings" },
  },
];

const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

export default router;
