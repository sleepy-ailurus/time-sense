<template>
  <div class="page-container">
    <div class="page-header">
      <h2 class="page-title">数据统计</h2>
      <div class="date-picker-wrap">
        <n-date-picker v-model:value="selectedDate" type="date" :clearable="false" />
      </div>
    </div>

    <div class="stats-grid">
      <div class="chart-card">
        <h3 class="chart-title">应用占比</h3>
        <div class="chart-container pie-chart" ref="appPieRef"></div>
      </div>
      <div class="chart-card">
        <h3 class="chart-title">分类占比</h3>
        <div class="chart-container pie-chart" ref="categoryPieRef"></div>
      </div>
    </div>

    <div class="chart-card">
      <h3 class="chart-title">近 7 天趋势</h3>
      <div class="chart-container trend-chart" ref="trendChartRef"></div>
    </div>

    <div class="chart-card">
      <h3 class="chart-title">时段分布</h3>
      <div class="chart-container hourly-chart" ref="hourlyChartRef"></div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { NDatePicker } from 'naive-ui'
import { format } from 'date-fns'
import { invoke } from '@tauri-apps/api/core'
import * as echarts from 'echarts'
import type { AppStat, CategoryStat, DailySummary, HourlyStat } from '../api/types'
import './timeline.css'

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
    return `${Math.floor(seconds / 60)} 分钟`
  }
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  if (minutes === 0) {
    return `${hours} 小时`
  }
  return `${hours}h ${minutes}m`
}

function formatDurationShort(seconds: number): string {
  if (seconds < 3600) {
    return `${Math.floor(seconds / 60)}分钟`
  }
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  return `${hours}h${minutes}m`
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
        return `<div style="font-weight: 600; margin-bottom: 4px;">${params.name}</div>
                <div>时长：${formatDuration(params.value)}</div>
                <div>占比：${pct}%</div>`
      },
    },
    legend: {
      show: false,
    },
    series: [
      {
        name: '应用占比',
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
          text: '应用总数',
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
    name: cat.categoryName,
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
        return `<div style="font-weight: 600; margin-bottom: 4px;">${params.name}</div>
                <div>时长：${formatDuration(params.value)}</div>
                <div>占比：${pct}%</div>`
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
        name: '分类占比',
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
            '{b|总时长}',
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

  const categoryConfig = [
    { key: 'workSeconds', name: '工作', color: '#3B82F6' },
    { key: 'studySeconds', name: '学习', color: '#10B981' },
    { key: 'entertainmentSeconds', name: '娱乐', color: '#EF4444' },
    { key: 'socialSeconds', name: '社交', color: '#8B5CF6' },
    { key: 'otherSeconds', name: '其他', color: '#6B7280' },
  ] as const

  // 过滤掉 7 天全为 0 的分类
  const visibleCategories = categoryConfig.filter((cat) =>
    weeklyTrend.value.some((d) => (d as any)[cat.key] > 0)
  )

  const seriesData = visibleCategories.map((cat) => {
    const data = weeklyTrend.value.map((d) => (d as any)[cat.key] / 60)
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
      name: '分钟',
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

  const weekDays = ['周一', '周二', '周三', '周四', '周五', '周六', '周日']
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
                <div>时长：${formatDuration(seconds)}</div>`
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
        name: '时段分布',
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
    const [apps, cats, trend, hourly] = await Promise.all([
      invoke<AppStat[]>('get_today_stats'),
      invoke<CategoryStat[]>('get_today_category_stats'),
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
</script>
