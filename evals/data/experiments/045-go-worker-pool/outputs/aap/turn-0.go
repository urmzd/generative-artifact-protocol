<aap:target id="worker-pool-package">package workerpool

import (
	"context"
	"fmt"
	"sync"
)

<aap:target id="types">
type Job interface {
	Execute(ctx context.Context) (interface{}, error)
}

type Result struct {
	Value interface{}
	Err   error
}

type PoolConfig struct {
	WorkerCount int
	JobQueueCap int
}

type WorkerPool struct {
	config  PoolConfig
	jobs    chan Job
	results chan Result
	wg      sync.WaitGroup
	ctx     context.Context
	cancel  context.CancelFunc
}
</aap:target>

<aap:target id="pool-implementation">
func New(config PoolConfig) *WorkerPool {
	ctx, cancel := context.WithCancel(context.Background())
	return &WorkerPool{
		config:  config,
		jobs:    make(chan Job, config.JobQueueCap),
		results: make(chan Result, config.JobQueueCap),
		ctx:     ctx,
		cancel:  cancel,
	}
}

func (p *WorkerPool) Start() {
	for i := 0; i < p.config.WorkerCount; i++ {
		p.wg.Add(1)
		go p.worker()
	}
}

func (p *WorkerPool) worker() {
	defer p.wg.Done()
	for {
		select {
		case <-p.ctx.Done():
			return
		case job, ok := <-p.jobs:
			if !ok {
				return
			}
			p.process(job)
		}
	}
}

func (p *WorkerPool) process(job Job) {
	defer func() {
		if r := recover(); r != nil {
			p.results <- Result{Err: fmt.Errorf("panic in worker: %v", r)}
		}
	}()
	res, err := job.Execute(p.ctx)
	p.results <- Result{Value: res, Err: err}
}

func (p *WorkerPool) Submit(job Job) {
	select {
	case p.jobs <- job:
	case <-p.ctx.Done():
	}
}

func (p *WorkerPool) Stop() {
	close(p.jobs)
	p.cancel()
	p.wg.Wait()
	close(p.results)
}

func (p *WorkerPool) Results() <-chan Result {
	return p.results
}
</aap:target>

<aap:target id="example-usage">
// Example: HTTP Request Processing
type HTTPJob struct {
	URL string
}

func (h *HTTPJob) Execute(ctx context.Context) (interface{}, error) {
	return fmt.Sprintf("Processed: %s", h.URL), nil
}

func Example() {
	pool := New(PoolConfig{WorkerCount: 3, JobQueueCap: 10})
	pool.Start()

	go func() {
		pool.Submit(&HTTPJob{URL: "http://example.com"})
		pool.Stop()
	}()

	for res := range pool.Results() {
		if res.Err != nil {
			fmt.Println("Error:", res.Err)
		} else {
			fmt.Println(res.Value)
		}
	}
}
</aap:target>
</aap:target>