<template>
  <div class="page-container">
    <div class="page-header">
      <h2 class="page-title">{{ t('heatmap.title') }}</h2>
      <div class="year-picker-wrap">
        <n-select v-model:value="selectedYear" :options="yearOptions" style="width: 120px" />
      </div>
    </div>
    <div class="stats-summary">
      <div class="stat-item">
        <div class="stat-value">{{ formatDuration(yearTotal) }}</div>
        <div class="stat-label">{{ t('heatmap.yearTotal') }}</div>
      </div>
      <div class="stat-item">
        <div class="stat-value">{{ avgPerDay }}</div>
        <div class="stat-label">{{ t('heatmap.avgPerDay') }}</div>
      </div>
      <div class="stat-item">
        <div class="stat-value">{{ maxStreak }}</div>
        <div class="stat-label">{{ t('heatmap.maxStreak') }}</div>
      </div>
    </div>
    <div class="chart-container heatmap-container" ref="chartRef"></div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { NSelect } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import * as echarts from 'echarts'
import type { HeatmapDay } from '../api/types'
import { formatDuration } from '../utils/format'
import './timeline.css'

const { t, tm, locale } = useI18n()

const selectedYear = ref(new Date().getFullYear())
const heatmapData = ref<HeatmapDay[]>([])
const chartRef = ref<HTMLElement | null>(null)
let chartInstance: echarts.ECharts | null = null

const yearOptions = computed(() => {
  const current = new Date().getFullYear()
  return Array.from({ length: 5 }, (_, i) => ({
    label: String(current - i),
    value: current - i,
  }))
})

const yearTotal = computed(() => {
  return heatmapData.value.reduce((sum, d) => sum + d.totalSeconds, 0)
})

const avgPerDay = computed(() => {
  const activeDays = heatmapData.value.filter(d => d.totalSeconds > 0).length
  if (activeDays === 0) return formatDuration(0)
  const avgSeconds = Math.round(yearTotal.value / activeDays)
  return formatDuration(avgSeconds)
})

const maxStreak = computed(() => {
  const validDays = heatmapData.value
    .filter(d => d.totalSeconds > 0)
    .map(d => d.date)
    .sort()

  if (validDays.length === 0) return 0

  let longest = 1
  let current = 1

  for (let i = 1; i < validDays.length; i++) {
    const prev = new Date(validDays[i - 1])
    const curr = new Date(validDays[i])
    const diff = Math.round((curr.getTime() - prev.getTime()) / (1000 * 60 * 60 * 24))

    if (diff === 1) {
      current++
      longest = Math.max(longest, current)
    } else {
      current = 1
    }
  }

  return longest
})

async function loadData() {
  try {
    const data = await invoke<HeatmapDay[]>('get_heatmap_data', { year: selectedYear.value })
    heatmapData.value = data
    await nextTick()
    renderChart()
  } catch (e) {
    console.error('Failed to load heatmap data:', e)
  }
}

function renderChart() {
  if (!chartRef.value) return

  if (!chartInstance) {
    chartInstance = echarts.init(chartRef.value)
  }

  const year = selectedYear.value
  const startDate = `${year}-01-01`
  const endDate = `${year}-12-31`

  // 转换为 ECharts heatmap 数据格式: [date, value, extraInfo...]
  const data = heatmapData.value.map(d => [d.date, d.totalSeconds, d.pomodoroCount])

  const option: echarts.EChartsOption = {
    tooltip: {
      backgroundColor: 'rgba(30, 30, 50, 0.95)',
      borderColor: 'rgba(255, 255, 255, 0.12)',
      borderWidth: 1,
      textStyle: {
        color: 'rgba(255, 255, 255, 0.95)',
        fontSize: 12,
      },
      formatter: function (params: any) {
        const date = params.value[0]
        const seconds = params.value[1] || 0
        const pomodoroCount = params.value[2] || 0
        return `
          <div style="font-weight: 600; margin-bottom: 4px;">${date}</div>
          <div>${t('heatmap.focusDuration')}: ${formatDuration(seconds)}</div>
          <div>${t('heatmap.pomodoroCount')}: ${pomodoroCount} ${t('common.unit')}</div>
        `
      },
    },
    visualMap: {
      show: true,
      orient: 'horizontal',
      left: 'right',
      bottom: 0,
      itemWidth: 12,
      itemHeight: 12,
      textStyle: {
        color: 'rgba(255, 255, 255, 0.6)',
        fontSize: 11,
      },
      pieces: [
        { min: 14400, label: t('heatmap.over4h'), color: '#0e4429' },
        { min: 7200, max: 14399, label: t('heatmap.h2to4'), color: '#006d32' },
        { min: 3600, max: 7199, label: t('heatmap.h1to2'), color: '#26a641' },
        { min: 1800, max: 3599, label: t('heatmap.min30to1h'), color: '#39d353' },
        { min: 1, max: 1799, label: t('heatmap.under30m'), color: '#9be9a8' },
        { value: 0, label: t('heatmap.none'), color: 'rgba(255, 255, 255, 0.06)' },
      ],
      text: [t('heatmap.more'), t('heatmap.less')],
    },
    calendar: {
      top: 40,
      left: 30,
      right: 20,
      bottom: 40,
      cellSize: [12, 12],
      range: [startDate, endDate],
      itemStyle: {
        color: 'rgba(255, 255, 255, 0.06)',
        borderWidth: 1,
        borderColor: 'rgba(255, 255, 255, 0.08)',
        borderRadius: 2,
      },
      splitLine: {
        show: false,
      },
      monthLabel: {
        show: true,
        nameMap: locale.value === 'zh' ? 'ZH' : 'EN',
        color: 'rgba(255, 255, 255, 0.6)',
        fontSize: 11,
        position: 'start',
      },
      dayLabel: {
        firstDay: 0,
        nameMap: tm('heatmap.weekdays') as string[],
        color: 'rgba(255, 255, 255, 0.6)',
        fontSize: 10,
      },
      yearLabel: {
        show: false,
      },
    },
    series: [
      {
        type: 'heatmap',
        coordinateSystem: 'calendar',
        data: data as any,
      },
    ],
  }

  chartInstance.setOption(option, true)
}

function handleResize() {
  chartInstance?.resize()
}

onMounted(() => {
  window.addEventListener('resize', handleResize)
  loadData()
})

onUnmounted(() => {
  window.removeEventListener('resize', handleResize)
  chartInstance?.dispose()
  chartInstance = null
})

watch(selectedYear, loadData)
watch(locale, () => renderChart())
</script>
