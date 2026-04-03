package workerpool

import (
	"container/heap"
	"context"
	"fmt"
	"sync"
)

type Priority int

const (
	PriorityLow    Priority = iota
	PriorityMedium
	PriorityHigh
)

type Job interface {
	Execute(ctx context.Context) (interface{}, error)
	Priority() Priority
}

type Result struct {
	Value interface{}
	Err   error
}

type item struct {
	job   Job
	index int
}

type priorityQueue []*item

func (pq priorityQueue) Len() int           { return len(pq) }
func (pq priorityQueue) Less(i, j int) bool { return pq[i].job.Priority() > pq[j].job.Priority() }
func (pq priorityQueue) Swap(i, j int)      { pq[i], pq[j] = pq[j], pq[i]; pq[i].index = i; pq[j].index = j }
func (pq *priorityQueue) Push(x interface{}) {
	n := len(*pq)
	item := x.(*item)
	item.index = n
	*pq = append(*pq, item)
}
func (pq *priorityQueue) Pop() interface{} {
	old := *pq
	n := len(old)
	item := old[n-1]
	old[n-1] = nil
	item.index = -1
	*pq = old[0 : n-1]
	return item
}

type WorkerPool struct {
	pq      priorityQueue
	jobs    chan Job
	results chan Result
	mu      sync.Mutex
	cond    *sync.Cond
	wg      sync.WaitGroup
	ctx     context.Context
	cancel  context.CancelFunc
	closed  bool
}

func New(numWorkers int, queueSize int) *WorkerPool {
	ctx, cancel := context.WithCancel(context.Background())
	p := &WorkerPool{
		jobs:    make(chan Job, queueSize),
		results: make(chan Result, queueSize),
		ctx:     ctx,
		cancel:  cancel,
	}
	p.cond = sync.NewCond(&p.mu)
	return p
}

func (p *WorkerPool) Start(numWorkers int) {
	for i := 0; i < numWorkers; i++ {
		p.wg.Add(1)
		go p.worker()
	}
	go p.orchestrator()
}

func (p *WorkerPool) orchestrator() {
	for {
		p.mu.Lock()
		for p.pq.Len() == 0 && !p.closed {
			p.cond.Wait()
		}
		if p.closed && p.pq.Len() == 0 {
			p.mu.Unlock()
			return
		}
		item := heap.Pop(&p.pq).(*item)
		p.mu.Unlock()

		select {
		case <-p.ctx.Done():
			return
		case p.jobs <- item.job:
		}
	}
}

func (p *WorkerPool) worker() {
	defer p.wg.Done()
	for job := range p.jobs {
		func() {
			defer func() {
				if r := recover(); r != nil {
					p.results <- Result{Err: fmt.Errorf("panic: %v", r)}
				}
			}()
			val, err := job.Execute(p.ctx)
			p.results <- Result{Value: val, Err: err}
		}()
	}
}

func (p *WorkerPool) Submit(job Job) {
	p.mu.Lock()
	defer p.mu.Unlock()
	heap.Push(&p.pq, &item{job: job})
	p.cond.Signal()
}

func (p *WorkerPool) Stop() {
	p.mu.Lock()
	p.closed = true
	p.cond.Broadcast()
	p.mu.Unlock()
	
	p.wg.Wait()
	close(p.jobs)
	close(p.results)
	p.cancel()
}

func (p *WorkerPool) Results() <-chan Result {
	return p.results
}