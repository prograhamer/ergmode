export interface TauriEvent<T> {
  event: string;
  payload: T;
}
