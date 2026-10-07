<template>
  <div class="page-container">
    <div class="page-header">
      <h2 class="page-title">时间轴</h2>
      <div class="date-picker-wrap">
        <n-date-picker v-model:value="selectedDate" type="date" :clearable="false" />
      </div>
    </div>
    <div class="chart-container" ref="chartRef"></div>
    <div v-if="loading" class="loading">加载中...</div>
    <div v-else-if="activities.length === 0" class="empty-state">
      <p>当天暂无活动记录</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { NDatePicker } from 'naive-ui'
import { format } from 'date-fns'
import { invoke } from '@tauri-apps/api/core'
import * as echarts from 'echarts'
import type { ActivityLog } from '../api/types'
import './timeline.css'

const selectedDate = ref<number>(Date.now())
const activities = ref<ActivityLog[]>([])
const loading = ref(false)
const chartRef = ref<HTMLElement | null>(null)
const chartInstance = ref<echarts.ECharts | null>(null)

// 分类顺序（从上到下）
const categoryOrder = [
  { id: 'work', name: '工作', color: '#3b82f6' },
  { id: 'study', name: '学习', color: '#22c55e' },
  { id: 'entertainment', name: '娱乐', color: '#ef4444' },
  { id: 'neutral', name: '中性', color: '#9ca3af' },
  { id: 'idle', name: '空闲', color: '#374151' },
]

function getCategoryInfo(activity: ActivityLog): { id: string; name: string; color: string; index: number } {
  if (activity.isIdle) {
    const idx = categoryOrder.findIndex(c => c.id === 'idle')
    return { ...categoryOrder[idx], index: idx }
  }
  const catMap: Record<number, string> = { 1: 'work', 2: 'study', 3: 'entertainment', 4: 'neutral' }
  const catId = activity.categoryId ? catMap[activity.categoryId] || 'neutral' : 'neutral'
  const idx = categoryOrder.findIndex(c => c.id === catId)
  return { ...categoryOrder[idx], index: idx }
}

function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  const s = seconds % 60
  if (h > 0) {
    return `${h}小时${m}分${s}秒`
  }
  if (m > 0) {
    return `${m}分${s}秒`
  }
  return `${s}秒`
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

  const yAxisData = categoryOrder.map(c => c.name)

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
              <span style="font-weight: 600; color: #fff;">${activity.processName}</span>
              <span style="color: #9ca3af; font-size: 11px;">[${catInfo.name}]</span>
            </div>
            ${activity.windowTitle ? `<div style="color: #9ca3af; margin-bottom: 6px; font-size: 11px; max-width: 240px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${activity.windowTitle}</div>` : ''}
            <div style="color: #d1d5db; font-size: 12px; line-height: 1.6;">
              <div>开始：${formatTime(activity.startTime)}</div>
              <div>结束：${formatTime(activity.endTime)}</div>
              <div>时长：${duration}</div>
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
    const data = await invoke<ActivityLog[]>('get_activity_by_date', { date: dateStr })
    activities.value = data
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
</script>
