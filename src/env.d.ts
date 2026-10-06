/// <reference types="vite/client" />

// 扩展 vue-router 的 RouteMeta 类型
declare module "vue-router" {
  interface RouteMeta {
    title?: string;
    icon?: string;
  }
}

export {};
