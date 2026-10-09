<template>
  <div class="page-container">
    <div class="page-header">
      <h2 class="page-title">{{ t('statistics.title') }}</h2>
      <div class="date-picker-wrap">
        <n-date-picker v-model:value="selectedDate" type="date" :clearable="false" />
      </div>
    </div>

    <div class="stats-grid">
      <div class="chart-card">
        <h3 class="chart-title">{{ t('statistics.appShare') }}</h3>
        <div class="chart-container pie-chart" ref="appPieRef"></div>
      </div>
      <div class="chart-card">
        <h3 class="chart-title">{{ t('statistics.categoryShare') }}</h3>
        <div class="chart-container pie-chart" ref="categoryPieRef"></div>
      </div>
    </div>

    <div class="chart-card">
      <h3 class="chart-title">{{ t('statistics.trend7') }}</h3>
      <div class="chart-container trend-chart" ref="trendChartRef"></div>
    </div>

    <div class="chart-card">
      <h3 class="chart-title">{{ t('statistics.hourly') }}</h3>
      <div class="chart-container hourly-chart" ref="hourlyChartRef"></div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { NDatePicker } from 'naive-ui'
import { format } from 'date-fns'
import { invoke } from '@tauri-apps/api/core'
import * as echarts from 'echarts'
import { escapeHtml } from '../utils/format'
import { categoryLabel } from '../utils/categoryName'
import type { AppStat, CategoryStat, DailySummary, HourlyStat } from '../api/types'
import './timeline.css'

const { t, tm, locale } = useI18n()

const selectedDate = ref<number>(Date.now())
const appStats = ref<AppStat[]>([])
const categoryStats = ref<CategoryStat[]>([])
const weeklyTrend = ref<DailySummary[]>([])
const hourlyData = ref<HourlyStat[]>([])

const appPieRef = ref<HTMLElement | null>(null)
const categoryPieRef = ref<HTMLElement | null>(null)
const trendChartRef = ref<HTMLElement | null>(null)
const hourlyChartRef = ref<HTMLElement | null>(null)

let appPieChart: echarts.ECharts | null = null
let categoryPieChart: echarts.ECharts | null = null
let trendChart: echarts.ECharts | null = null
let hourlyChart: echarts.ECharts | null = null

// ========== 工具函数 ==========

function formatDuration(seconds: number): string {
  if (seconds < 3600) {
    return t('common.durationMinutes', { m: Math.floor(seconds / 60) })
  }
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  if (minutes === 0) {
    return t('common.durationHours', { h: hours })
  }
  return t('common.durationHourMin', { h: hours, m: minutes })
}

function formatDurationShort(seconds: number): string {
  if (seconds < 3600) {
    return t('common.durationMinShort', { m: Math.floor(seconds / 60) })
  }
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  return t('common.durationHourMinShort', { h: hours, m: minutes })
}

// 渐变色系（暗色主题适配）
const pieGradientColors = [
  ['#5B8FF9', '#27727B'],
  ['#61DDAA', '#159A86'],
  ['#65789B', '#3E5474'],
  ['#F6BD16', '#E69600'],
  ['#7262FD', '#4B3FD4'],
  ['#E8684A', '#C94A2C'],
  ['#5DD8E0', '#2BB5BE'],
  ['#FF9845', '#E07A20'],
  ['#945FB9', '#723E97'],
  ['#35C7C4', '#1AA5A2'],
]

function getPieColor(index: number): echarts.graphic.LinearGradient {
  const colorPair = pieGradientColors[index % pieGradientColors.length]
  return new echarts.graphic.LinearGradient(0, 0, 1, 1, [
    { offset: 0, color: colorPair[0] },
    { offset: 1, color: colorPair[1] },
  ])
}

// ========== 图表 1：应用占比饼图 ==========

function renderAppPieChart() {
  if (!appPieRef.value) return
  if (!appPieChart) {
    appPieChart = echarts.init(appPieRef.value)
  }

  const data = appStats.value.map((app, index) => ({
    name: app.processName,
    value: app.totalSeconds,
    itemStyle: {
      color: getPieColor(index),
    },
  }))

  const totalSeconds = appStats.value.reduce((sum, a) => sum + a.totalSeconds, 0)

  const option: echarts.EChartsOption = {
    backgroundColor: 'transparent',
    tooltip: {
      trigger: 'item',
      backgroundColor: 'rgba(30, 30, 40, 0.95)',
      borderColor: 'rgba(255, 255, 255, 0.1)',
      textStyle: {
        color: '#e0e0e0',
      },
      formatter: (params: any) => {
        const pct = params.percent != null ? params.percent.toFixed(1) : '0'
        return `<div style="font-weight: 600; margin-bottom: 4px;">${escapeHtml(params.name)}</div>
                <div>${t('statistics.duration')}：${formatDuration(params.value)}</div>
                <div>${t('statistics.percent')}：${pct}%</div>`
      },
    },
    legend: {
      show: false,
    },
    series: [
      {
        name: t('statistics.appShare'),
        type: 'pie',
        radius: ['50%', '75%'],
        center: ['50%', '50%'],
        avoidLabelOverlap: true,
        itemStyle: {
          borderRadius: 6,
          borderColor: 'rgba(20, 20, 26, 0.8)',
          borderWidth: 2,
        },
        label: {
          show: true,
          position: 'outside',
          color: '#c0c0c0',
          fontSize: 11,
          formatter: '{b}',
        },
        labelLine: {
          show: true,
          lineStyle: {
            color: 'rgba(255, 255, 255, 0.3)',
          },
        },
        emphasis: {
          label: {
            show: true,
            fontSize: 13,
            fontWeight: 'bold',
            color: '#ffffff',
          },
          itemStyle: {
            shadowBlur: 20,
            shadowOffsetX: 0,
            shadowColor: 'rgba(0, 0, 0, 0.5)',
          },
        },
        data,
      },
    ],
    graphic: [
      {
        type: 'text',
        left: 'center',
        top: '42%',
        style: {
          text: appStats.value.length.toString(),
          align: 'center',
          fill: '#ffffff',
          fontSize: 28,
          fontWeight: 'bold',
        },
      },
      {
        type: 'text',
        left: 'center',
        top: '55%',
        style: {
          text: t('statistics.appCount'),
          align: 'center',
          fill: '#a0a0a0',
          fontSize: 12,
        },
      },
      {
        type: 'text',
        left: 'center',
        top: '65%',
        style: {
          text: formatDurationShort(totalSeconds),
          align: 'center',
          fill: '#61DDAA',
          fontSize: 13,
          fontWeight: 500,
        },
      },
    ],
  }

  appPieChart.setOption(option, true)
}

// ========== 图表 2：分类占比饼图 ==========

function renderCategoryPieChart() {
  if (!categoryPieRef.value) return
  if (!categoryPieChart) {
    categoryPieChart = echarts.init(categoryPieRef.value)
  }

  const data = categoryStats.value.map((cat) => ({
    name: categoryLabel(cat.categoryName),
    value: cat.totalSeconds,
    itemStyle: {
      color: cat.categoryColor,
    },
  }))

  const totalSeconds = categoryStats.value.reduce((sum, c) => sum + c.totalSeconds, 0)

  const option: echarts.EChartsOption = {
    backgroundColor: 'transparent',
    tooltip: {
      trigger: 'item',
      backgroundColor: 'rgba(30, 30, 40, 0.95)',
      borderColor: 'rgba(255, 255, 255, 0.1)',
      textStyle: {
        color: '#e0e0e0',
      },
      formatter: (params: any) => {
        const pct = params.percent != null ? params.percent.toFixed(1) : '0'
        return `<div style="font-weight: 600; margin-bottom: 4px;">${escapeHtml(params.name)}</div>
                <div>${t('statistics.duration')}：${formatDuration(params.value)}</div>
                <div>${t('statistics.percent')}：${pct}%</div>`
      },
    },
    legend: {
      orient: 'vertical',
      right: 5,
      top: 'center',
      textStyle: {
        color: '#c0c0c0',
        fontSize: 12,
      },
      itemWidth: 10,
      itemHeight: 10,
      itemGap: 10,
    },
    series: [
      {
        name: t('statistics.categoryShare'),
        type: 'pie',
        radius: ['55%', '78%'],
        center: ['40%', '50%'],
        avoidLabelOverlap: true,
        itemStyle: {
          borderRadius: 6,
          borderColor: 'rgba(20, 20, 26, 0.8)',
          borderWidth: 2,
        },
        label: {
          show: true,
          position: 'center',
          formatter: [
            `{a|${formatDurationShort(totalSeconds)}}`,
            `{b|${t('statistics.totalDuration')}}`,
          ].join('\n'),
          rich: {
            a: {
              color: '#fff',
              fontSize: 18,
              fontWeight: 'bold',
              lineHeight: 24,
              align: 'center',
            },
            b: {
              color: '#a0a0a0',
              fontSize: 11,
              lineHeight: 18,
              align: 'center',
            },
          },
        },
        emphasis: {
          label: {
            show: true,
            formatter: (params: any) => {
              const pct = params.percent != null ? params.percent.toFixed(1) : '0'
              return `${params.name}\n${pct}%`
            },
            fontSize: 14,
            fontWeight: 'bold',
            color: '#ffffff',
          },
          itemStyle: {
            shadowBlur: 20,
            shadowOffsetX: 0,
            shadowColor: 'rgba(0, 0, 0, 0.5)',
          },
        },
        data,
      },
    ],
  }

  categoryPieChart.setOption(option, true)
}

// ========== 图表 3：近 7 天趋势折线图 ==========

function renderTrendChart() {
  if (!trendChartRef.value) return
  if (!trendChart) {
    trendChart = echarts.init(trendChartRef.value)
  }

  const dates = weeklyTrend.value.map((d) => {
    const parts = d.date.split('-')
    return `${parts[1]}/${parts[2]}`
  })

  // 分类维度：优先用后端返回的分类明细（含用户自建分类），
  // 老数据（没有明细字段）才回退到固定的 5 个内置分类
  type TrendCategory = { name: string; color: string; valueOf: (d: DailySummary) => number }

  const hasBreakdown = weeklyTrend.value.some((d) => (d.categorySeconds?.length ?? 0) > 0)

  const trendCategories: TrendCategory[] = hasBreakdown
    ? (() => {
        const meta = new Map<number, { name: string; color: string }>()
        for (const day of weeklyTrend.value) {
          for (const slice of day.categorySeconds ?? []) {
            if (!meta.has(slice.categoryId)) {
              meta.set(slice.categoryId, {
                name: categoryLabel(slice.categoryName),
                color: slice.categoryColor || '#6B7280',
              })
            }
          }
        }
        return [...meta.entries()]
          .map(([id, m]) => ({
            name: m.name,
            color: m.color,
            valueOf: (d: DailySummary) =>
              (d.categorySeconds ?? []).find((s) => s.categoryId === id)?.seconds ?? 0,
          }))
          .sort((a, b) =>
            Math.max(...weeklyTrend.value.map(b.valueOf)) - Math.max(...weeklyTrend.value.map(a.valueOf)),
          )
      })()
    : [
        { name: t('statistics.work'), color: '#3B82F6', valueOf: (d: DailySummary) => d.workSeconds },
        { name: t('statistics.study'), color: '#10B981', valueOf: (d: DailySummary) => d.studySeconds },
        { name: t('statistics.entertainment'), color: '#EF4444', valueOf: (d: DailySummary) => d.entertainmentSeconds },
        { name: t('statistics.social'), color: '#8B5CF6', valueOf: (d: DailySummary) => d.socialSeconds },
        { name: t('statistics.other'), color: '#6B7280', valueOf: (d: DailySummary) => d.otherSeconds },
      ]

  // 过滤掉 7 天全为 0 的分类
  const visibleCategories = trendCategories.filter((cat) =>
    weeklyTrend.value.some((d) => cat.valueOf(d) > 0)
  )

  const seriesData = visibleCategories.map((cat) => {
    const data = weeklyTrend.value.map((d) => cat.valueOf(d) / 60)
    return {
      name: cat.name,
      type: 'line',
      smooth: true,
      symbol: 'circle',
      symbolSize: 6,
      showSymbol: false,
      emphasis: {
        focus: 'series',
      },
      lineStyle: {
        width: 2,
        color: cat.color,
      },
      areaStyle: {
        color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
          { offset: 0, color: cat.color + '66' },
          { offset: 1, color: cat.color + '05' },
        ]),
      },
      itemStyle: {
        color: cat.color,
      },
      data,
    }
  })

  const option: echarts.EChartsOption = {
    backgroundColor: 'transparent',
    tooltip: {
      trigger: 'axis',
      backgroundColor: 'rgba(30, 30, 40, 0.95)',
      borderColor: 'rgba(255, 255, 255, 0.1)',
      textStyle: {
        color: '#e0e0e0',
      },
      axisPointer: {
        type: 'line',
        lineStyle: {
          color: 'rgba(255, 255, 255, 0.2)',
        },
      },
      formatter: (params: any) => {
        if (!Array.isArray(params) || params.length === 0) return ''
        let html = `<div style="font-weight: 600; margin-bottom: 8px;">${params[0].axisValue}</div>`
        params.forEach((item: any) => {
          const minutes = item.value
          const durationStr = formatDuration(minutes * 60)
          html += `<div style="display: flex; align-items: center; gap: 8px; margin: 4px 0;">
            <span style="display: inline-block; width: 10px; height: 10px; border-radius: 50%; background: ${item.color};"></span>
            <span>${item.seriesName}：${durationStr}</span>
          </div>`
        })
        return html
      },
    },
    legend: {
      data: visibleCategories.map((c) => c.name),
      textStyle: {
        color: '#c0c0c0',
        fontSize: 12,
      },
      top: 10,
      itemWidth: 16,
      itemHeight: 8,
      itemGap: 20,
    },
    grid: {
      left: 50,
      right: 20,
      top: 50,
      bottom: 30,
    },
    xAxis: {
      type: 'category',
      boundaryGap: false,
      data: dates,
      axisLine: {
        lineStyle: {
          color: 'rgba(255, 255, 255, 0.15)',
        },
      },
      axisLabel: {
        color: '#a0a0a0',
        fontSize: 11,
      },
      axisTick: {
        show: false,
      },
    },
    yAxis: {
      type: 'value',
      name: t('statistics.minutes'),
      nameTextStyle: {
        color: '#808080',
        fontSize: 11,
      },
      axisLine: {
        show: false,
      },
      axisLabel: {
        color: '#a0a0a0',
        fontSize: 11,
      },
      splitLine: {
        lineStyle: {
          color: 'rgba(255, 255, 255, 0.08)',
          type: 'dashed',
        },
      },
    },
    series: seriesData as any,
  }

  trendChart.setOption(option, true)
}

// ========== 图表 4：时段分布热力图 ==========

function renderHourlyChart() {
  if (!hourlyChartRef.value) return
  if (!hourlyChart) {
    hourlyChart = echarts.init(hourlyChartRef.value)
  }

  const weekDays = tm('statistics.weekdays') as string[]
  const hours = Array.from({ length: 24 }, (_, i) => `${i}:00`)

  // 构建 24小时 x 7天 的热力图数据
  const heatmapData: [number, number, number][] = []
  let maxSeconds = 0

  hourlyData.value.forEach((item) => {
    if (item.totalSeconds > maxSeconds) {
      maxSeconds = item.totalSeconds
    }
    heatmapData.push([item.hour, item.weekday, item.totalSeconds])
  })

  const option: echarts.EChartsOption = {
    backgroundColor: 'transparent',
    tooltip: {
      backgroundColor: 'rgba(30, 30, 40, 0.95)',
      borderColor: 'rgba(255, 255, 255, 0.1)',
      textStyle: {
        color: '#e0e0e0',
      },
      formatter: (params: any) => {
        const hour = params.data[0]
        const weekday = params.data[1]
        const seconds = params.data[2]
        return `<div style="font-weight: 600; margin-bottom: 4px;">${weekDays[weekday]} ${hour}:00 - ${hour + 1}:00</div>
                <div>${t('statistics.duration')}：${formatDuration(seconds)}</div>`
      },
    },
    grid: {
      left: 70,
      right: 30,
      top: 15,
      bottom: 50,
    },
    xAxis: {
      type: 'category',
      data: hours,
      splitArea: {
        show: false,
      },
      axisLine: {
        show: false,
      },
      axisLabel: {
        color: '#9ca3af',
        fontSize: 11,
        interval: 3,
      },
      axisTick: {
        show: false,
      },
    },
    yAxis: {
      type: 'category',
      data: weekDays,
      axisLine: {
        show: false,
      },
      axisLabel: {
        color: '#9ca3af',
        fontSize: 12,
      },
      axisTick: {
        show: false,
      },
      splitArea: {
        show: false,
      },
    },
    visualMap: {
      min: 0,
      max: maxSeconds > 0 ? maxSeconds : 3600,
      calculable: false,
      orient: 'horizontal',
      left: 'center',
      bottom: 0,
      itemWidth: 15,
      itemHeight: 120,
      inRange: {
        color: ['#1e293b', '#334155', '#3b82f6', '#60a5fa', '#93c5fd'],
      },
      textStyle: {
        color: '#6b7280',
        fontSize: 10,
      },
      formatter: (value: any) => formatDurationShort(value),
    },
    series: [
      {
        name: t('statistics.hourly'),
        type: 'heatmap',
        data: heatmapData,
        label: {
          show: false,
        },
        emphasis: {
          itemStyle: {
            shadowBlur: 10,
            shadowColor: 'rgba(0, 0, 0, 0.5)',
          },
        },
        itemStyle: {
          borderRadius: 4,
          borderWidth: 2,
          borderColor: 'rgba(17, 24, 39, 0.8)',
        },
      },
    ],
  }

  hourlyChart.setOption(option, true)
}

// ========== 统一渲染入口 ==========

function renderCharts() {
  renderAppPieChart()
  renderCategoryPieChart()
  renderTrendChart()
  renderHourlyChart()
}

// ========== 窗口 resize 处理 ==========

function handleResize() {
  appPieChart?.resize()
  categoryPieChart?.resize()
  trendChart?.resize()
  hourlyChart?.resize()
}

// ========== 生命周期 ==========

async function loadData() {
  try {
    const dateStr = format(selectedDate.value, 'yyyy-MM-dd')
    const [apps, cats, trend, hourly] = await Promise.all([
      invoke<AppStat[]>('get_today_stats', { date: dateStr }),
      invoke<CategoryStat[]>('get_today_category_stats', { date: dateStr }),
      invoke<DailySummary[]>('get_weekly_trend'),
      invoke<HourlyStat[]>('get_hourly_distribution'), // 不传 date，查过去 7 天
    ])
    appStats.value = apps
    categoryStats.value = cats
    weeklyTrend.value = trend
    hourlyData.value = hourly
    await nextTick()
    renderCharts()
  } catch (e) {
    console.error('Failed to load statistics data:', e)
  }
}

onMounted(() => {
  loadData()
  window.addEventListener('resize', handleResize)
})

onUnmounted(() => {
  window.removeEventListener('resize', handleResize)
  appPieChart?.dispose()
  categoryPieChart?.dispose()
  trendChart?.dispose()
  hourlyChart?.dispose()
})

watch(selectedDate, loadData)
// 语言切换后图表内的文字（series 名、tooltip、坐标轴）需要重绘
watch(locale, () => renderCharts())
</script>
