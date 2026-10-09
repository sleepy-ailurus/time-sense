<template>
  <div class="page-container">
    <div class="page-header">
      <h2 class="page-title">{{ t('timeline.title') }}</h2>
      <div class="date-picker-wrap">
        <n-date-picker v-model:value="selectedDate" type="date" :clearable="false" />
      </div>
    </div>
    <div class="chart-container" ref="chartRef"></div>
    <div v-if="loading" class="loading">{{ t('common.loading') }}</div>
    <div v-else-if="activities.length === 0" class="empty-state">
      <p>{{ t('timeline.empty') }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { NDatePicker } from 'naive-ui'
import { format } from 'date-fns'
import { invoke } from '@tauri-apps/api/core'
import * as echarts from 'echarts'
import { escapeHtml } from '../utils/format'
import { categoryLabel } from '../utils/categoryName'
import { getCategories } from '../api'
import type { ActivityLog, Category } from '../api/types'
import './timeline.css'

const { t, locale } = useI18n()

const selectedDate = ref<number>(Date.now())
const activities = ref<ActivityLog[]>([])
const categories = ref<Category[]>([])
const loading = ref(false)
const chartRef = ref<HTMLElement | null>(null)
const chartInstance = ref<echarts.ECharts | null>(null)

// 时间轴每一行 = 真实分类（用后端返回的名称/颜色，顺序按分类表）+ 空闲
const categoryRows = computed(() => {
  const list = categories.value.map((c) => ({
    key: `cat-${c.id}`,
    categoryId: c.id as number | null,
    rawName: c.name,
    name: categoryLabel(c.name),
    color: c.color || '#6B7280',
  }))

  // 没有分类的记录统一落到「其他」那一行；只有连「其他」分类都不存在时才补一行
  const hasUncategorized = activities.value.some(
    (a) => !a.isIdle && !list.some((r) => r.categoryId === a.categoryId),
  )
  const hasOther = list.some((r) => r.rawName === '其他' || r.rawName === 'Other')
  if (hasUncategorized && !hasOther) {
    list.push({
      key: 'other',
      categoryId: null,
      rawName: '其他',
      name: t('category.other'),
      color: '#6B7280',
    })
  }

  list.push({
    key: 'idle',
    categoryId: null,
    rawName: '',
    name: t('timeline.idle'),
    color: '#374151',
  })
  return list
})

/** 未分类（或分类已被删除）的记录归入「其他」行 */
const otherRowIndex = computed(() =>
  categoryRows.value.findIndex((r) => r.rawName === '其他' || r.rawName === 'Other'),
)

function getCategoryInfo(activity: ActivityLog): { key: string; name: string; color: string; index: number } {
  const rows = categoryRows.value
  if (activity.isIdle) {
    const idleIdx = rows.findIndex((r) => r.key === 'idle')
    return { ...rows[idleIdx], index: idleIdx }
  }
  const idx = rows.findIndex((r) => r.categoryId != null && r.categoryId === activity.categoryId)
  const index = idx >= 0 ? idx : Math.max(otherRowIndex.value, 0)
  return { ...rows[index], index }
}

function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  const s = seconds % 60
  if (h > 0) {
    return t('common.durationHourMinSec', { h, m, s })
  }
  if (m > 0) {
    return t('common.durationMinSec', { m, s })
  }
  return t('common.durationSeconds', { n: s })
}

function formatTime(timestamp: number): string {
  return format(new Date(timestamp * 1000), 'HH:mm:ss')
}

function getDayRange(dateTs: number): { start: number; end: number } {
  const d = new Date(dateTs)
  const start = new Date(d.getFullYear(), d.getMonth(), d.getDate(), 0, 0, 0).getTime()
  const end = new Date(d.getFullYear(), d.getMonth(), d.getDate(), 23, 59, 59).getTime()
  return { start, end }
}

function renderChart() {
  if (!chartRef.value || activities.value.length === 0) {
    if (chartInstance.value) {
      chartInstance.value.dispose()
      chartInstance.value = null
    }
    return
  }

  // 初始化或复用实例
  if (!chartInstance.value) {
    chartInstance.value = echarts.init(chartRef.value, undefined, { renderer: 'canvas' })
  }

  const { start: dayStart, end: dayEnd } = getDayRange(selectedDate.value)

  // 构建 custom series 数据
  // 数据格式: [startTime (ms), yCategoryIndex, duration (ms), activityIndex]
  const seriesData = activities.value.map((activity, index) => {
    const startMs = activity.startTime * 1000
    const endMs = activity.endTime * 1000
    const catInfo = getCategoryInfo(activity)
    return {
      value: [startMs, catInfo.index, endMs - startMs, index],
      itemStyle: {
        color: catInfo.color,
      },
      raw: activity,
    }
  })

  const yAxisData = categoryRows.value.map(c => c.name)

  const option: echarts.EChartsOption = {
    backgroundColor: 'transparent',
    grid: {
      left: 70,
      right: 20,
      top: 20,
      bottom: 40,
    },
    tooltip: {
      backgroundColor: 'rgba(17, 24, 39, 0.95)',
      borderColor: 'rgba(255, 255, 255, 0.1)',
      borderWidth: 1,
      textStyle: {
        color: '#e5e7eb',
        fontSize: 12,
      },
      formatter: (params: any) => {
        const activity = params.data.raw as ActivityLog
        const duration = formatDuration(activity.duration)
        const catInfo = getCategoryInfo(activity)
        return `
          <div style="padding: 4px 0;">
            <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 6px;">
              <span style="display: inline-block; width: 10px; height: 10px; border-radius: 2px; background: ${catInfo.color};"></span>
              <span style="font-weight: 600; color: #fff;">${escapeHtml(activity.siteLabel || activity.processName)}</span>
              <span style="color: #9ca3af; font-size: 11px;">[${catInfo.name}]</span>
            </div>
            ${activity.windowTitle ? `<div style="color: #9ca3af; margin-bottom: 6px; font-size: 11px; max-width: 240px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${escapeHtml(activity.windowTitle)}</div>` : ''}
            <div style="color: #d1d5db; font-size: 12px; line-height: 1.6;">
              <div>${t('timeline.start')}：${formatTime(activity.startTime)}</div>
              <div>${t('timeline.end')}：${formatTime(activity.endTime)}</div>
              <div>${t('timeline.duration')}：${duration}</div>
            </div>
          </div>
        `
      },
    },
    xAxis: {
      type: 'time',
      min: dayStart,
      max: dayEnd,
      axisLine: {
        lineStyle: {
          color: 'rgba(255, 255, 255, 0.1)',
        },
      },
      axisLabel: {
        color: '#9ca3af',
        fontSize: 11,
        formatter: (value: number) => format(value, 'HH:mm'),
      },
      splitLine: {
        lineStyle: {
          color: 'rgba(255, 255, 255, 0.05)',
          type: 'dashed',
        },
      },
    },
    yAxis: {
      type: 'category',
      data: yAxisData,
      axisLine: {
        show: false,
      },
      axisTick: {
        show: false,
      },
      axisLabel: {
        color: '#9ca3af',
        fontSize: 12,
      },
      splitLine: {
        show: true,
        lineStyle: {
          color: 'rgba(255, 255, 255, 0.04)',
          type: 'dashed',
        },
      },
    },
    series: [
      {
        type: 'custom',
        renderItem: (params: any, api: any) => {
          const start = api.coord([api.value(0), api.value(1)])
          const end = api.coord([api.value(0) + api.value(2), api.value(1)])
          const barHeight = api.size([0, 1])[1] * 0.6
          const rectY = start[1] - barHeight / 2

          return {
            type: 'rect',
            shape: {
              x: start[0],
              y: rectY,
              width: Math.max(end[0] - start[0], 1),
              height: barHeight,
              r: 3,
            },
            style: api.style({
              fill: api.style().fill,
              stroke: 'rgba(255, 255, 255, 0.08)',
              lineWidth: 1,
            }),
            styleEmphasis: {
              fill: api.style().fill,
              stroke: 'rgba(255, 255, 255, 0.3)',
              lineWidth: 1,
              shadowBlur: 8,
              shadowColor: 'rgba(0, 0, 0, 0.3)',
            },
          }
        },
        encode: {
          x: [0, 2],
          y: 1,
        },
        data: seriesData,
      },
    ],
  }

  chartInstance.value.setOption(option, true)
}

function handleResize() {
  if (chartInstance.value) {
    chartInstance.value.resize()
  }
}

async function loadData() {
  loading.value = true
  try {
    const dateStr = format(selectedDate.value, 'yyyy-MM-dd')
    const [data, cats] = await Promise.all([
      invoke<ActivityLog[]>('get_activity_by_date', { date: dateStr }),
      getCategories(),
    ])
    activities.value = data
    categories.value = cats
    await nextTick()
    renderChart()
  } catch (e) {
    console.error('Failed to load activity data:', e)
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  loadData()
  window.addEventListener('resize', handleResize)
})

onUnmounted(() => {
  window.removeEventListener('resize', handleResize)
  if (chartInstance.value) {
    chartInstance.value.dispose()
    chartInstance.value = null
  }
})

watch(selectedDate, loadData)
watch(locale, () => renderChart())
</script>
