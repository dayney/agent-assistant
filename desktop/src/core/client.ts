import type { ApplyPreview, WorkspaceSnapshot } from './model';

export interface CoreClient {
    getSnapshot(): Promise<WorkspaceSnapshot>;
    previewApply(): Promise<ApplyPreview>;
}
