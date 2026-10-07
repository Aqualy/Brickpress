/** Serialize atomic records and remember failures until that record is successfully retried. */
export class PersistenceQueue {
  private tail: Promise<unknown> = Promise.resolve();
  private failures = new Map<string, unknown>();
  run<T>(key: string, task: () => Promise<T>): Promise<T> {
    const pending = this.tail.then(task).then(
      (value) => {
        this.failures.delete(key);
        return value;
      },
      (error) => {
        this.failures.set(key, error);
        throw error;
      }
    );
    this.tail = pending.catch(() => undefined);
    return pending;
  }
  async flush() {
    await this.tail;
    if (this.failures.size) throw this.failures.values().next().value;
  }
}
