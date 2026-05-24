export class RingBuffer {
  private _cap: number;
  private _data: Float32Array;
  private _writeIdx: number;
  private _size: number;

  constructor(size: number) {
    this._cap = size;

    this._data = new Float32Array(size);

    this._writeIdx = 0;
    this._size = size;
  }

  push(value: number) {
    this._data[this._writeIdx] = value;
    this._writeIdx = (this._writeIdx + 1) % this._cap;
    if (this._size < this._cap) {
      this._size++;
    }
  }

  read(i: number) {
    return this._data[(this._writeIdx + i) % this._cap];
  }

  get raw() {
    return this._data;
  }

  get writeIdx() {
    return this._writeIdx;
  }

  get length() {
    return this._size;
  }
}

export class RingBuffer2D {
    private _ringBuffer: RingBuffer;
    private _binCount: number;
    private _frameCount: number;

    constructor(public binCount: number, public frameCount: number) {
        this._binCount = binCount;
        this._frameCount = frameCount;

        this._ringBuffer = new RingBuffer(binCount * frameCount);
    }

    push(frame: number[]) {
        for(let i = 0; i < this._binCount; i++) {
            this._ringBuffer.push(frame[i]);
        }
    }

    forEach(callback: (frame: Float32Array, x: number) => void) {
        for(let x = 0; x < this._frameCount; x++) {
            const startIndex = (this._ringBuffer.writeIdx + x * this._binCount) % this._ringBuffer.length;
            callback(this._ringBuffer.raw.subarray(startIndex, startIndex + this._binCount), x);
        }
    }
}