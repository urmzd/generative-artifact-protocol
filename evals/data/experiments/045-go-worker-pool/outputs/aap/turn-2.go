<aap:target id="worker-pool-package">package workerpool

import (
	"context"
	"fmt"
	"sync"
)

<aap:target id="types">
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
	
	// Internal priority queues
	highPrio := make([]Job, 0)
	medPrio := make([]Job, 0)
	lowPrio := make([]Job, 0)

	for {
		// Attempt to drain channel if empty
		if len(highPrio) == 0 && len(medPrio) == 0 && len(lowPrio) == 0 {
			select {
			case <-p.ctx.Done():
				return
			case job, ok := <-p.jobs:
				if !ok {
					return
				}
				p.queueJob(&highPrio, &medPrio, &lowPrio, job)
			}
		}

		// Non-blocking drain remaining jobs
		for {
			select {
			case job, ok := <-p.jobs:
				if !ok {
					// Jobs closed, process everything else
					p.drainAndProcess(&highPrio, &medPrio, &lowPrio)
					return
				}
				p.queueJob(&highPrio, &medPrio, &lowPrio, job)
			default:
				// Process highest available
				if len(highPrio) > 0 {
					p.process(p.pop(&highPrio))
				} else if len(medPrio) > 0 {
					p.process(p.pop(&medPrio))
				} else if len(lowPrio) > 0 {
					p.process(p.pop(&lowPrio))
				} else {
					goto next
				}
			}
		}
	next:
	}
}

func (p *WorkerPool) queueJob(h, m, l *[]Job, job Job) {
	switch job.Priority() {
	case PriorityHigh:
		*h = append(*h, job)
	case PriorityMedium:
		*m = append(*m, job)
	default:
		*l = append(*l, job)
	}
}

func (p *WorkerPool) pop(s *[]Job) Job {
	job := (*s)[0]
	*s = (*s)[1:]
	return job
}

func (p *WorkerPool) drainAndProcess(h, m, l *[]Job) {
	for len(*h) > 0 { p.process(p.pop(h)) }
	for len(*m) > 0 { p.process(p.pop(m)) }
	for len(*l) > 0 { p.process(p.pop(l)) }
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
// Example: Image Resize Processing
type ResizeJob struct {
	Filename string
	Width    int
	Height   int
	Prio     Priority
}

func (j *ResizeJob) Execute(ctx context.Context) (interface{}, error) {
	return fmt.Sprintf("Resized %s to %dx%d", j.Filename, j.Width, j.Height), nil
}

func (j *ResizeJob) Priority() Priority {
	return j.Prio
}

func Example() {
	pool := New(PoolConfig{WorkerCount: 4, JobQueueCap: 20})
	pool.Start()

	go func() {
		for i := 1; i <= 20; i++ {
			pool.Submit(&ResizeJob{
				Filename: fmt.Sprintf("image_%d.jpg", i),
				Width:    800,
				Height:   600,
				Prio:     PriorityMedium,
			})
		}
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
