package workerpool

import (
	"context"
	"fmt"
	"sync"
)

type Job interface {
	Execute(ctx context.Context) (interface{}, error)
}

type Result struct {
	Value interface{}
	Err   error
}

type PoolConfig struct {
	NumWorkers int
	QueueSize  int
}

type WorkerPool struct {
	config  PoolConfig
	jobs    chan Job
	results chan Result
	wg      sync.WaitGroup
	cancel  context.CancelFunc
	ctx     context.Context
}

func New(config PoolConfig) *WorkerPool {
	ctx, cancel := context.WithCancel(context.Background())
	return &WorkerPool{
		config:  config,
		jobs:    make(chan Job, config.QueueSize),
		results: make(chan Result, config.QueueSize),
		ctx:     ctx,
		cancel:  cancel,
	}
}

func (p *WorkerPool) Start() {
	for i := 0; i < p.config.NumWorkers; i++ {
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
}

func (p *WorkerPool) Submit(job Job) {
	select {
	case <-p.ctx.Done():
		return
	case p.jobs <- job:
	}
}

func (p *WorkerPool) Results() <-chan Result {
	return p.results
}

func (p *WorkerPool) Stop() {
	close(p.jobs)
	p.cancel()
	p.wg.Wait()
	close(p.results)
}

// Example usage implementation

type HTTPJob struct {
	URL string
}

func (j *HTTPJob) Execute(ctx context.Context) (interface{}, error) {
	// Simulate HTTP Request
	return fmt.Sprintf("Response from %s", j.URL), nil
}

func Example() {
	pool := New(PoolConfig{NumWorkers: 3, QueueSize: 10})
	pool.Start()

	urls := []string{"http://google.com", "http://github.com", "http://golang.org"}

	go func() {
		for _, url := range urls {
			pool.Submit(&HTTPJob{URL: url})
		}
		pool.Stop()
	}()

	for res := range pool.Results() {
		if res.Err != nil {
			fmt.Printf("Error: %v\n", res.Err)
		} else {
			fmt.Printf("Success: %v\n", res.Value)
		}
	}
}