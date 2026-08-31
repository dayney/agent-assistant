import type { CapabilityState, HealthState } from '../core/model';

type StatusValue = CapabilityState | HealthState | 'synced' | 'discovered' | 'attention' | 'not-configured';

const labels: Record<StatusValue, string> = {
    full: '完整',
    partial: '部分支持',
    unsupported: '不支持',
    good: '健康',
    warn: '需关注',
    bad: '异常',
    synced: '已同步',
    discovered: '已发现',
    attention: '需处理',
    'not-configured': '未配置',
};

export function StatusBadge({ value }: { value: StatusValue }) {
    return <span className={`status-badge status-${value}`}><span aria-hidden="true" />{labels[value]}</span>;
}
